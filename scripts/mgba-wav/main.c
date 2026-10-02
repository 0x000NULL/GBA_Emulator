/* mgba-wav: run a ROM headlessly in libmgba and write its audio as a 16-bit stereo WAV.
 *
 *   mgba-wav <rom.gba> <out.wav> <frames> <rate> [inputs.txt]
 *
 * `rate` is the output sample rate (our harness writes 32768 Hz, or 65536 Hz for games that set
 * SOUNDBIAS to that resolution; use the rate the `--pcm` capture reports so the two files line
 * up sample for sample). `inputs.txt` holds lines `frame mask`: from that frame on the GBA keys
 * in `mask` are held, in KEYINPUT bit order (A=1 B=2 Select=4 Start=8 Right=10h Left=20h Up=40h
 * Down=80h R=100h L=200h), until the next line. The run starts from reset with no BIOS image, so
 * mGBA's HLE BIOS is used, like ours.
 *
 * Written against libmgba's public core API only (include/mgba/core/core.h, config.h,
 * blip_buf.h); nothing here depends on how the core emulates anything. */

#include <mgba/core/blip_buf.h>
#include <mgba/core/config.h>
#include <mgba/core/core.h>
#include <mgba/core/log.h>

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct Step {
	unsigned frame;
	uint32_t mask;
};

/* Only fatal and error log lines reach stderr. */
static void quietLog(struct mLogger* logger, int category, enum mLogLevel level, const char* format, va_list args) {
	(void) logger;
	if (level & (mLOG_FATAL | mLOG_ERROR)) {
		fprintf(stderr, "mgba[%s]: ", mLogCategoryName(category));
		vfprintf(stderr, format, args);
		fputc('\n', stderr);
	}
}

static void putU16(FILE* f, unsigned v) {
	unsigned char b[2] = { v & 0xFF, (v >> 8) & 0xFF };
	fwrite(b, 1, 2, f);
}

static void putU32(FILE* f, uint32_t v) {
	unsigned char b[4] = { v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, (v >> 24) & 0xFF };
	fwrite(b, 1, 4, f);
}

/* 44-byte RIFF header for 16-bit stereo PCM, same layout as gba-test's write_wav. */
static void writeHeader(FILE* f, uint32_t rate, uint32_t sampleFrames) {
	uint32_t dataBytes = sampleFrames * 4;
	fseek(f, 0, SEEK_SET);
	fwrite("RIFF", 1, 4, f);
	putU32(f, 36 + dataBytes);
	fwrite("WAVEfmt ", 1, 8, f);
	putU32(f, 16);
	putU16(f, 1);
	putU16(f, 2);
	putU32(f, rate);
	putU32(f, rate * 4);
	putU16(f, 4);
	putU16(f, 16);
	fwrite("data", 1, 4, f);
	putU32(f, dataBytes);
}

static struct Step* readSchedule(const char* path, size_t* count) {
	FILE* f = fopen(path, "r");
	if (!f) {
		fprintf(stderr, "mgba-wav: cannot open %s\n", path);
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

int main(int argc, char** argv) {
	if (argc < 5 || argc > 6) {
		fprintf(stderr, "usage: mgba-wav <rom.gba> <out.wav> <frames> <rate> [inputs.txt]\n");
		return 2;
	}
	const char* romPath = argv[1];
	const char* wavPath = argv[2];
	unsigned frames = (unsigned) strtoul(argv[3], NULL, 10);
	unsigned rate = (unsigned) strtoul(argv[4], NULL, 10);
	size_t stepCount = 0;
	struct Step* steps = argc == 6 ? readSchedule(argv[5], &stepCount) : NULL;
	if (!frames || !rate) {
		fprintf(stderr, "mgba-wav: frames and rate must be positive\n");
		return 2;
	}

	static struct mLogger logger = { .log = quietLog };
	mLogSetDefaultLogger(&logger);

	struct mCore* core = mCoreFind(romPath);
	if (!core) {
		fprintf(stderr, "mgba-wav: %s is not a ROM libmgba recognises\n", romPath);
		return 2;
	}
	core->init(core);
	unsigned width;
	unsigned height;
	core->desiredVideoDimensions(core, &width, &height);
	color_t* video = calloc((size_t) width * height, sizeof(color_t));
	core->setVideoBuffer(core, video, width);

	/* An in-memory configuration only: no user config file, HLE BIOS, no idle-loop skipping. */
	mCoreConfigInit(&core->config, NULL);
	mCoreConfigSetValue(&core->config, "useBios", "0");
	mCoreConfigSetValue(&core->config, "skipBios", "0");
	mCoreConfigSetValue(&core->config, "idleOptimization", "ignore");
	/* the core maps these into its options; without them the master volume is zero */
	mCoreConfigSetIntValue(&core->config, "volume", 0x100);
	mCoreConfigSetIntValue(&core->config, "mute", 0);
	mCoreConfigSetUIntValue(&core->config, "sampleRate", rate);
	mCoreConfigSetUIntValue(&core->config, "audioBuffers", 0x4000);
	mCoreLoadConfig(core);

	if (!mCoreLoadFile(core, romPath)) {
		fprintf(stderr, "mgba-wav: cannot load %s\n", romPath);
		return 2;
	}
	core->setAudioBufferSize(core, 0x4000);
	core->reset(core);
	double clock = core->frequency(core);
	blip_t* left = core->getAudioChannel(core, 0);
	blip_t* right = core->getAudioChannel(core, 1);
	blip_set_rates(left, clock, rate);
	blip_set_rates(right, clock, rate);

	FILE* wav = fopen(wavPath, "wb");
	if (!wav) {
		fprintf(stderr, "mgba-wav: cannot write %s\n", wavPath);
		return 2;
	}
	writeHeader(wav, rate, 0);

	static short buf[2 * 4096];
	uint32_t written = 0;
	size_t next = 0;
	uint32_t keys = 0;
	for (unsigned frame = 0; frame < frames; ++frame) {
		while (next < stepCount && steps[next].frame <= frame) {
			keys = steps[next].mask;
			++next;
		}
		core->setKeys(core, keys);
		core->runFrame(core);
		int avail = blip_samples_avail(left);
		int availRight = blip_samples_avail(right);
		if (availRight < avail) {
			avail = availRight;
		}
		while (avail > 0) {
			int n = avail > 4096 ? 4096 : avail;
			blip_read_samples(left, buf, n, 1);
			blip_read_samples(right, buf + 1, n, 1);
			fwrite(buf, 4, n, wav);
			written += n;
			avail -= n;
		}
	}
	writeHeader(wav, rate, written);
	fclose(wav);
	printf("mgba-wav: %u frames, %u samples at %u Hz -> %s\n", frames, written, rate, wavPath);

	core->deinit(core);
	free(video);
	free(steps);
	return 0;
}
