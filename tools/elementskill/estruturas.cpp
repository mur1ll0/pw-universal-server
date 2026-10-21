// B313 (fase 1 do DLL de dados): o tamanho das estruturas que o executável passa ao elementskill.dll, provado no
// binário com página de guarda. Cada estrutura é posta colada no fim de uma página cuja seguinte é proibida; chama-se
// a função do DLL para toda habilidade (níveis 1 e máximo). Ler além do tamanho = violação de acesso.
// Com o tamanho do fonte não pode haver violação; com 4 bytes a menos, se o último campo é lido, há.
// Uso: estruturas.exe <elementskill.dll>
#include <windows.h>
#include <stdio.h>
#include <string.h>

typedef void(__cdecl* FnVoid)();
typedef unsigned int(__cdecl* FnNext)(unsigned int);
typedef int(__cdecl* FnReq)(unsigned int, void*, int);

static char* pagina;  // 2 páginas: a segunda sem acesso
static DWORD tam_pagina;

// Chama `f(id, req, nivel)` com a estrutura de `tam` bytes (conteúdo `modelo`) colada no fim da página.
static int chamar(FnReq f, unsigned int id, const void* modelo, int tam, int nivel, int* falhou) {
    char* p = pagina + tam_pagina - tam;
    memcpy(p, modelo, tam);
    __try { *falhou = 0; return f(id, p, nivel); }
    __except (GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) { *falhou = 1; return 0; }
}

int main(int argc, char** argv) {
    HMODULE dll = LoadLibraryA(argv[1]);
    if (!dll) return 1;
    FnVoid init = (FnVoid)GetProcAddress(dll, "?InitStaticData@ElementSkill@GNET@@SAXXZ");
    FnNext next = (FnNext)GetProcAddress(dll, "?NextSkill@ElementSkill@GNET@@SAII@Z");
    if (init) init();
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    tam_pagina = si.dwPageSize;
    pagina = (char*)VirtualAlloc(NULL, 2 * tam_pagina, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
    DWORD velho;
    VirtualProtect(pagina + tam_pagina, tam_pagina, PAGE_NOACCESS, &velho);

    // Conteúdo plausível: tudo "suficiente" (nível, SP, dinheiro, mana altos), para o código seguir adiante e ler
    // todos os campos. A lista do PetRequirement aponta para um vetor válido.
    static int lista[64] = {0};
    int modelo[64];
    for (int i = 0; i < 64; i++) modelo[i] = 100000;
    struct { const char* nome; const char* sym; int tam_fonte; int ponteiro_em; } t[] = {
        // ElementSkill.h do fonte 1.5.5 (iguais no 1.5.3): LearnRequirement 6 int; UseRequirement 7 int + bool (4 com
        // alinhamento) + 2 int + ComboSkillState (uint + 3 int); PetRequirement 2 int + int* + int;
        // GoblinRequirement int[5] + 5 int; GoblinUseRequirement 2 int + int[5] + 3 int.
        {"LearnRequirement", "?LearnCondition@ElementSkill@GNET@@SAHIAAULearnRequirement@2@H@Z", 24, -1},
        {"UseRequirement", "?Condition@ElementSkill@GNET@@SAHIAAUUseRequirement@2@H@Z", 56, -1},
        {"PetRequirement", "?PetLearn@ElementSkill@GNET@@SAHIAAUPetRequirement@2@H@Z", 16, 8},
        {"GoblinRequirement", "?GoblinLearn@ElementSkill@GNET@@SAHIAAUGoblinRequirement@2@H@Z", 40, -1},
        {"GoblinUseRequirement", "?GoblinCondition@ElementSkill@GNET@@SAHIAAUGoblinUseRequirement@2@H@Z", 44, -1},
    };
    for (auto& e : t) {
        FnReq f = (FnReq)GetProcAddress(dll, e.sym);
        if (!f) { printf("%s: sem export\n", e.nome); continue; }
        int m[64];
        memcpy(m, modelo, sizeof(m));
        if (e.ponteiro_em >= 0) { *(int**)((char*)m + e.ponteiro_em) = lista; *(int*)((char*)m + e.ponteiro_em + 4) = 0; }
        // UseRequirement: o bool is_combat (offset 28) falso, a forma (8) 0, a arma (12) 0.
        if (!strcmp(e.nome, "UseRequirement")) { m[2] = 0; m[3] = 0; m[7] = 0; m[10] = 0; }
        for (int delta = 0; delta <= 4; delta += 4) {
            int tam = e.tam_fonte - delta, violacoes = 0, chamadas = 0, primeiro = 0;
            for (unsigned int id = next(0); id; id = next(id)) {
                for (int nivel = 1; nivel <= 2; nivel++) {
                    int falhou;
                    chamar(f, id, m, tam, nivel, &falhou);
                    chamadas++;
                    if (falhou && !violacoes++) primeiro = id;
                }
            }
            printf("%-22s tamanho %2d: %d chamadas, %d violações%s", e.nome, tam, chamadas, violacoes, violacoes ? "" : "\n");
            if (violacoes) printf(" (primeira na habilidade %d)\n", primeiro);
        }
    }
    return 0;
}
