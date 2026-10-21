// B323: a vtable do stub de cada habilidade (RVA de 48 entradas), para achar o GetIntroduction de cada uma no
// binário (tools/elementskill/argumentos_da_descricao.py). Uso: vtable_dos_stubs.exe <elementskill.dll> > stubs.txt
#include <windows.h>
#include <stdio.h>
int main(int c, char** a) {
  HMODULE d = LoadLibraryA(a[1]);
  ((void(__cdecl*)())GetProcAddress(d, "?InitStaticData@ElementSkill@GNET@@SAXXZ"))();
  void*(__cdecl* cr)(unsigned, int) = (void*(__cdecl*)(unsigned, int))GetProcAddress(d, "?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
  unsigned(__cdecl* nx)(unsigned) = (unsigned(__cdecl*)(unsigned))GetProcAddress(d, "?NextSkill@ElementSkill@GNET@@SAII@Z");
  for (unsigned id = nx(0); id; id = nx(id)) {
    void* s = cr(id, 1); void* stub = *(void**)((char*)s + 4); void** vt = *(void***)stub;
    printf("%u", id);
    for (int k = 0; k < 48; k++) printf(" %lx", (unsigned long)((char*)vt[k] - (char*)d));
    printf("\n");
  }
}
