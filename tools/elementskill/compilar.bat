@echo off
rem Compila numeros.exe (32 bits, como o elementskill.dll). Precisa do Visual Studio 2022 com C++ (x86).
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars32.bat" >nul
cd /d "%~dp0"
cl /nologo /O2 /EHsc numeros.cpp /Fe:numeros.exe
del numeros.obj 2>nul
