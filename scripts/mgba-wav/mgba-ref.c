/* mgba-ref: run a ROM headlessly in libmgba and write what a comparison with gba-test needs:
 * pictures, memory words, per-frame registers and an instruction trace.
 *
 *   mgba-ref <rom.gba> <frames> [options]
 *     --input FILE          `frame mask` schedule (same format as mgba-wav)
 *     --png F[,F...] PREFIX write the picture after frame F (1-based, like gba-test --frames F)
 *                           as PREFIX-F.ppm (binary PPM, 8-bit RGB)
 *     --dump LO..HI         at the end, print `mem %08X: %08X` lines (gba-test --dump-mem format)
 *     --regs FILE           after every frame: `frame cycle r0..r14 pc cpsr` plus key IO halfwords
 *     --poll A[,A...]       step every instruction and print `poll frame cycle pc addr old new`
 *                           whenever one of these IO/memory halfwords changes (hex addresses)
 *     --trace-pc FROM..TO FILE  the same window as `cycle pc` lines (pc = the instruction about to
 *                           run; gba-test --trace-hash prints the same first two columns)
 *     --trace FROM..TO FILE one line per executed instruction in frames FROM..TO (0-based), in
 *                           gba-test's `--trace` layout, prefixed by the global cycle count:
 *                           `cycle r0 .. r14 pc+pipe cpsr: XXXXXXXX`
 *
 * The run starts from reset with no BIOS image (mGBA's HLE BIOS), no idle-loop skipping.
 *
 * Written against libmgba's public core API only (include/mgba/core/core.h, config.h, timing.h,
 * interface.h); nothing here depends on how the core emulates anything. */

#include <mgba/core/config.h>
#include <mgba/core/core.h>
#include <mgba/core/interface.h>
#include <mgba/core/log.h>
#include <mgba/core/timing.h>

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct Step {
	unsigned frame;
	uint32_t mask;
};

static void quietLog(struct mLogger* logger, int category, enum mLogLevel level, const char* format, va_list args) {
	(void) logger;
	const char* name = mLogCategoryName(category);
	if (name && strstr(name, "Debug")) {
		/* the guest's debug-print registers (mgba-suite reports its score this way) */
		fputs("guest: ", stdout);
		vprintf(format, args);
		fputc('\n', stdout);
		return;
	}
	if (level & (mLOG_FATAL | mLOG_ERROR)) {
		fprintf(stderr, "mgba[%s]: ", mLogCategoryName(category));
		vfprintf(stderr, format, args);
		fputc('\n', stderr);
	}
}

static struct Step* readSchedule(const char* path, size_t* count) {
	FILE* f = fopen(path, "r");
	if (!f) {
		fprintf(stderr, "mgba-ref: cannot open %s\n", path);
		exit(2);
	}
	size_t cap = 64;
	size_t n = 0;
	struct Step* steps = malloc(cap * sizeof(*steps));
	char line[256];
	while (fgets(line, sizeof(line), f)) {
		unsigned frame;
		unsigned mask;
		if (line[0] == '#' || sscanf(line, "%u %i", &frame, &mask) != 2) {
			continue;
		}
		if (n == cap) {
			cap *= 2;
			steps = realloc(steps, cap * sizeof(*steps));
		}
		steps[n].frame = frame;
		steps[n].mask = mask & 0x3FF;
		++n;
	}
	fclose(f);
	*count = n;
	return steps;
}

static int parseRange(const char* s, uint32_t* lo, uint32_t* hi) {
	const char* dots = strstr(s, "..");
	if (!dots) {
		return 0;
	}
	*lo = (uint32_t) strtoul(s, NULL, 16);
	*hi = (uint32_t) strtoul(dots + 2, NULL, 16);
	return 1;
}

static uint32_t reg(struct mCore* core, const char* name) {
	uint32_t v = 0;
	core->readRegister(core, name, &v);
	return v;
}

/* Cycles since reset. This libmgba keeps the running total in masterCycles (globalCycles stays 0)
 * plus the cycles the CPU has run since the last event. */
static uint64_t now(struct mCore* core) {
	return (uint64_t) core->timing->masterCycles + (core->timing->relativeCycles ? *core->timing->relativeCycles : 0);
}

static const char* REG_NAMES[16] = {
	"r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7",
	"r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
};

/* Between instructions libmgba reports r15 one instruction ahead of the next PC (reset: r15 = 4
 * with the next instruction at 0); gba-test prints the next PC plus 8 (ARM) / 4 (Thumb). */
static const int32_t pcBiasArm = 4;
static const int32_t pcBiasThumb = 2;

/* The address of the next instruction. */
static uint32_t nextPc(struct mCore* core) {
	return reg(core, "r15") - ((reg(core, "cpsr") & 0x20) ? 2 : 4);
}

static void traceLine(FILE* f, struct mCore* core) {
	uint32_t cpsr = reg(core, "cpsr");
	fprintf(f, "%llu ", (unsigned long long) now(core));
	for (int i = 0; i < 15; ++i) {
		fprintf(f, "%08X ", reg(core, REG_NAMES[i]));
	}
	uint32_t pc = reg(core, "r15") + ((cpsr & 0x20) ? pcBiasThumb : pcBiasArm);
	fprintf(f, "%08X cpsr: %08X\n", pc, cpsr);
}

static const uint32_t IO_WORDS[] = {
	0x04000000, /* DISPCNT */
	0x04000004, /* DISPSTAT */
	0x04000006, /* VCOUNT */
	0x04000100, 0x04000102, 0x04000104, 0x04000106, /* TM0, TM1 counter/control */
	0x04000108, 0x0400010A, 0x0400010C, 0x0400010E,
	0x040000BA, 0x040000C6, 0x040000D2, 0x040000DE, /* DMA0-3 CNT_H */
	0x04000200, 0x04000202, 0x04000208, /* IE IF IME */
	0x04000130, /* KEYINPUT */
};

static void writePpm(const char* path, const color_t* video, unsigned width, unsigned height) {
	FILE* f = fopen(path, "wb");
	if (!f) {
		fprintf(stderr, "mgba-ref: cannot write %s\n", path);
		exit(2);
	}
	fprintf(f, "P6\n%u %u\n255\n", width, height);
	for (size_t i = 0; i < (size_t) width * height; ++i) {
		uint32_t c = video[i];
		unsigned char rgb[3] = { c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF };
		fwrite(rgb, 1, 3, f);
	}
	fclose(f);
}

static void usage(void) {
	fprintf(stderr, "usage: mgba-ref <rom.gba> <frames> [--input FILE] [--png F[,F...] PREFIX] [--dump LO..HI]... [--regs FILE] [--poll A[,A...]] [--trace FROM..TO FILE]\n");
	exit(2);
}

int main(int argc, char** argv) {
	if (argc < 3) {
		usage();
	}
	const char* romPath = argv[1];
	unsigned frames = (unsigned) strtoul(argv[2], NULL, 10);
	size_t stepCount = 0;
	struct Step* steps = NULL;
	unsigned pngFrames[256];
	size_t pngCount = 0;
	const char* pngPrefix = NULL;
	uint32_t dumps[32][2];
	size_t dumpCount = 0;
	FILE* regsOut = NULL;
	FILE* traceOut = NULL;
	int tracePcOnly = 0;
	uint32_t traceFrom = 0;
	uint32_t traceTo = 0;
	uint32_t polls[64];
	uint32_t pollPrev[64];
	size_t pollCount = 0;
	for (int i = 3; i < argc; ++i) {
		if (!strcmp(argv[i], "--input") && i + 1 < argc) {
			steps = readSchedule(argv[++i], &stepCount);
		} else if (!strcmp(argv[i], "--png") && i + 2 < argc) {
			char* list = argv[++i];
			for (char* t = strtok(list, ","); t && pngCount < 256; t = strtok(NULL, ",")) {
				pngFrames[pngCount++] = (unsigned) strtoul(t, NULL, 10);
			}
			pngPrefix = argv[++i];
		} else if (!strcmp(argv[i], "--dump") && i + 1 < argc && dumpCount < 32) {
			if (!parseRange(argv[++i], &dumps[dumpCount][0], &dumps[dumpCount][1])) {
				usage();
			}
			++dumpCount;
		} else if (!strcmp(argv[i], "--poll") && i + 1 < argc) {
			char* list = argv[++i];
			for (char* t = strtok(list, ","); t && pollCount < 64; t = strtok(NULL, ",")) {
				polls[pollCount++] = (uint32_t) strtoul(t, NULL, 16);
			}
		} else if (!strcmp(argv[i], "--regs") && i + 1 < argc) {
			regsOut = fopen(argv[++i], "w");
		} else if ((!strcmp(argv[i], "--trace") || !strcmp(argv[i], "--trace-pc")) && i + 2 < argc) {
			tracePcOnly = !strcmp(argv[i], "--trace-pc");
			uint32_t lo;
			uint32_t hi;
			const char* r = argv[++i];
			const char* dots = strstr(r, "..");
			if (!dots) {
				usage();
			}
			lo = (uint32_t) strtoul(r, NULL, 10);
			hi = (uint32_t) strtoul(dots + 2, NULL, 10);
			traceFrom = lo;
			traceTo = hi;
			traceOut = fopen(argv[++i], "w");
		} else {
			usage();
		}
	}
	if (!frames) {
		usage();
	}

	static struct mLogger logger = { .log = quietLog };
	mLogSetDefaultLogger(&logger);

	struct mCore* core = mCoreFind(romPath);
	if (!core) {
		fprintf(stderr, "mgba-ref: %s is not a ROM libmgba recognises\n", romPath);
		return 2;
	}
	core->init(core);
	unsigned width;
	unsigned height;
	core->desiredVideoDimensions(core, &width, &height);
	color_t* video = calloc((size_t) width * height, sizeof(color_t));
	core->setVideoBuffer(core, video, width);

	mCoreConfigInit(&core->config, NULL);
	mCoreConfigSetValue(&core->config, "useBios", "0");
	mCoreConfigSetValue(&core->config, "skipBios", "0");
	mCoreConfigSetValue(&core->config, "idleOptimization", "ignore");
	mCoreConfigSetIntValue(&core->config, "volume", 0x100);
	mCoreConfigSetIntValue(&core->config, "mute", 0);
	mCoreLoadConfig(core);
	if (!mCoreLoadFile(core, romPath)) {
		fprintf(stderr, "mgba-ref: cannot load %s\n", romPath);
		return 2;
	}
	core->reset(core);


	for (size_t p = 0; p < pollCount; ++p) {
		pollPrev[p] = core->rawRead16(core, polls[p], -1);
	}
	size_t next = 0;
	uint32_t keys = 0;
	for (unsigned frame = 0; frame < frames; ++frame) {
		while (next < stepCount && steps[next].frame <= frame) {
			keys = steps[next].mask;
			++next;
		}
		core->setKeys(core, keys);
		int tracing = traceOut && frame >= traceFrom && frame <= traceTo;
		if (tracing || pollCount) {
			uint32_t f0 = core->frameCounter(core);
			while (core->frameCounter(core) == f0) {
				if (tracing && tracePcOnly) {
					fprintf(traceOut, "%llu %08X\n", (unsigned long long) now(core), nextPc(core));
				} else if (tracing) {
					traceLine(traceOut, core);
				}
				core->step(core);
				for (size_t p = 0; p < pollCount; ++p) {
					uint32_t v = core->rawRead16(core, polls[p], -1);
					if (v != pollPrev[p]) {
						uint32_t cpsr = reg(core, "cpsr");
						printf("poll %u %llu %08X %08X %04X %04X\n", frame, (unsigned long long) now(core),
						       reg(core, "r15") + ((cpsr & 0x20) ? pcBiasThumb : pcBiasArm), polls[p], pollPrev[p], v);
						pollPrev[p] = v;
					}
				}
			}
		} else {
			core->runFrame(core);
		}
		if (regsOut) {
			fprintf(regsOut, "%u %llu", frame + 1, (unsigned long long) now(core));
			uint32_t cpsr = reg(core, "cpsr");
			for (int i = 0; i < 15; ++i) {
				fprintf(regsOut, " %08X", reg(core, REG_NAMES[i]));
			}
			fprintf(regsOut, " %08X %08X", reg(core, "r15") + ((cpsr & 0x20) ? pcBiasThumb : pcBiasArm), cpsr);
			for (size_t i = 0; i < sizeof(IO_WORDS) / sizeof(IO_WORDS[0]); ++i) {
				fprintf(regsOut, " %04X", core->rawRead16(core, IO_WORDS[i], -1));
			}
			fputc('\n', regsOut);
		}
		for (size_t i = 0; i < pngCount; ++i) {
			if (pngFrames[i] == frame + 1) {
				char path[1024];
				snprintf(path, sizeof(path), "%s-%u.ppm", pngPrefix, frame + 1);
				writePpm(path, video, width, height);
				printf("ppm=%s\n", path);
			}
		}
	}
	for (size_t d = 0; d < dumpCount; ++d) {
		for (uint32_t a = dumps[d][0] & ~3u; a <= dumps[d][1]; a += 4) {
			printf("mem %08X: %08X\n", a, core->rawRead32(core, a, -1));
			if (a + 4 == 0) {
				break;
			}
		}
	}
	printf("mgba-ref: %u frames, cycle=%llu\n", frames, (unsigned long long) now(core));
	if (regsOut) {
		fclose(regsOut);
	}
	if (traceOut) {
		fclose(traceOut);
	}
	core->deinit(core);
	free(video);
	free(steps);
	return 0;
}
