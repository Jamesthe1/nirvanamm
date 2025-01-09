#!/bin/bash

if [ ! -d xdelta/build ]; then mkdir xdelta/build; fi
cd xdelta/build
cmake -DBUILD_SHARED_LIBS=ON -DXD3_ENABLE_SECONDARY_COMPRESSION=ON -DXD3_ENABLE_ENCODER=ON -DXD3_ENABLE_VCDIFF_TOOLS=ON -DXD3_ENABLE_LZMA=ON -DCMAKE_BUILD_TYPE=Release -G "MinGW Makefiles" ..
make -j
mv libxdelta3.dll ../../libxdelta3.dll
cd -