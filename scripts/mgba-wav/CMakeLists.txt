# mgba-wav: a headless audio capture tool built on libmgba (the reference half of the audio
# comparison, see scripts/compare-audio.py). scripts/build-mgba.ps1 drives this file.
#
# MGBA_SRC points at an mGBA checkout (the script clones tag 0.10.5 into tools/mgba-src). Only the
# library is built, in its dependency-free static configuration (LIBMGBA_ONLY: no frontends, no
# zlib/libpng/ffmpeg, no debugger, no scripting), GBA core only.
cmake_minimum_required(VERSION 3.16)
project(mgba-wav C)

if(NOT DEFINED MGBA_SRC)
    set(MGBA_SRC "${CMAKE_CURRENT_SOURCE_DIR}/../../tools/mgba-src")
endif()
get_filename_component(MGBA_SRC "${MGBA_SRC}" ABSOLUTE)
if(NOT EXISTS "${MGBA_SRC}/CMakeLists.txt")
    message(FATAL_ERROR "mGBA source not found at ${MGBA_SRC} (run scripts/build-mgba.ps1, which clones it)")
endif()

set(LIBMGBA_ONLY ON)
set(M_CORE_GBA ON)
set(M_CORE_GB OFF)
set(USE_DEBUGGERS OFF)
set(ENABLE_SCRIPTING OFF)
set(USE_LUA OFF)
set(BUILD_GL OFF)
set(BUILD_GLES2 OFF)
set(BUILD_GLES3 OFF)
add_subdirectory("${MGBA_SRC}" mgba EXCLUDE_FROM_ALL)

add_executable(mgba-wav main.c)
target_link_libraries(mgba-wav mgba)

# mgba-ref: pictures, memory, per-frame registers and instruction traces for scripts/compare-frames.py
add_executable(mgba-ref mgba-ref.c)
target_link_libraries(mgba-ref mgba)
