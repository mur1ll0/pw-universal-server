@echo off
rem B317: compila o nosso elementskill.dll (x86). /Zc:wchar_t- = wchar_t como unsigned short (VC6), para os nomes
rem exportados saírem iguais aos que o ELEMENTCLIENT.EXE importa; /MT = sem depender do runtime do VS no jogador.
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars32.bat" >nul
cd /d "%~dp0"
cl /nologo /O2 /EHsc /MT /Zc:wchar_t- /LD elementskill_dados.cpp /Fe:elementskill.dll /link /NOLOGO
del *.obj *.exp 2>nul
