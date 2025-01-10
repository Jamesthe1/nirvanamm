@echo off

if NOT EXIST DIR xdelta\build then mkdir xdelta\build
cd xdelta\build

cmake -DBUILD_SHARED_LIBS=ON -DXD3_ENABLE_SECONDARY_COMPRESSION=ON -DXD3_ENABLE_ENCODER=ON -DXD3_ENABLE_VCDIFF_TOOLS=ON -DXD3_ENABLE_LZMA=ON -DCMAKE_BUILD_TYPE=Release ..
msbuild xdelta3.sln /p:Configuration=Release
cd Release

copy ..\..\..\xdelta3.def .
lib /def:xdelta3.def /out:xdelta3.lib /machine:x64
copy xdelta3.lib ..\..\..
copy xdelta3.dll ..\..\..\..

cd ..\..\..