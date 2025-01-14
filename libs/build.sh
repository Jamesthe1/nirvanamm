#!/bin/bash

case $1 in
    xz)
        cd xz/windows
        if [ -d build ]; then rm -r build; fi
        CMAKE_DIR=$(which cmake)
        GCC_DIR=$(which gcc)
        ./build_with_cmake.bat $(dirname CMAKE_DIR) $(dirname CMAKE_DIR) ON
        mv build/liblzma.dll ../../..
        cd -
        ;;
    xdelta)
        if [ ! -d xdelta/build ]; then mkdir xdelta/build; fi
        cd xdelta/build
        cmake -DBUILD_SHARED_LIBS=ON -DXD3_ENABLE_SECONDARY_COMPRESSION=ON -DXD3_ENABLE_ENCODER=ON -DXD3_ENABLE_VCDIFF_TOOLS=ON -DXD3_ENABLE_LZMA=ON -DCMAKE_BUILD_TYPE=Release -G "MinGW Makefiles" ..
        make -j
        mv libxdelta3.dll ../../..
        cd -
        ;;
    bridge)
        cd xdelta-bridge
        make
        cd -
        ;;
esac