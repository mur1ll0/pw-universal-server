// B303/B304: lê os números das habilidades do elementskill.dll instalado no cliente (o binário é o juiz).
// Uso: numeros.exe <elementskill.dll> [niveis.txt]  → uma linha JSON por (habilidade, nível) na saída.
//
// Os índices da tabela virtual são descobertos no próprio binário: a vtable da base (??_7ElementSkill@GNET@@6B@,
// exportada) aponta para as funções da base, também exportadas — o endereço de cada uma dá o índice. Só vale slot
// cujo endereço é único na vtable: o compilador do 1.2.6 fundiu as funções idênticas da base (22 slots com o mesmo
// `return 0`), e aí o endereço não diz qual é qual — o campo fica de fora (avisado na saída de erro).
// Os requisitos e o tempo de execução vêm também pelas estáticas exportadas `GetRequired*(id, nível)` e
// `GetExecuteTime(id, nível)`, que fazem a chamada virtual certa por conta própria (prefixo `s_`).
// `niveis.txt` (linhas "id nível_máximo"): para o DLL que não exporta `GetMaxLevel` (1.2.6).
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <map>

typedef void(__cdecl* FnVoid)();
typedef unsigned int(__cdecl* FnNext)(unsigned int);
typedef void*(__cdecl* FnCreate)(unsigned int, int);
typedef int(__cdecl* FnIdNivel)(unsigned int, int);
typedef int(__thiscall* FnInt)(void*);
typedef void(__thiscall* FnDestroy)(void*);

static HMODULE dll;
static void* sym(const char* n) { return (void*)GetProcAddress(dll, n); }

const int TAM = 64;

// Índice, na vtable da base, da função de base exportada com o nome `n`; -1 se ausente ou se o endereço se repete.
static int indice(void** vt, const char* n) {
    void* f = sym(n);
    if (!f) return -1;
    int achado = -1, vezes = 0;
    for (int i = 0; i < TAM; i++) if (vt[i] == f) { achado = i; vezes++; }
    return vezes == 1 ? achado : -2;
}

int main(int argc, char** argv) {
    if (argc < 2) { fprintf(stderr, "uso: numeros.exe elementskill.dll [niveis.txt]\n"); return 2; }
    dll = LoadLibraryA(argv[1]);
    if (!dll) { fprintf(stderr, "LoadLibrary falhou: %lu\n", GetLastError()); return 1; }
    FnVoid init = (FnVoid)sym("?InitStaticData@ElementSkill@GNET@@SAXXZ");
    FnNext next = (FnNext)sym("?NextSkill@ElementSkill@GNET@@SAII@Z");
    FnCreate criar = (FnCreate)sym("?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
    FnDestroy destruir = (FnDestroy)sym("?Destroy@ElementSkill@GNET@@QAEXXZ");
    void** vt = (void**)sym("??_7ElementSkill@GNET@@6B@");
    if (!next || !criar || !destruir || !vt) { fprintf(stderr, "faltam exports essenciais\n"); return 1; }
    std::map<unsigned int, int> niveis;
    if (argc > 2) {
        FILE* f = fopen(argv[2], "r");
        unsigned int id; int n;
        while (f && fscanf(f, "%u %d", &id, &n) == 2) niveis[id] = n;
        if (f) fclose(f);
        fprintf(stderr, "níveis máximos do arquivo: %zu\n", niveis.size());
    }
    struct { const char* chave; const char* nome; int i; } v[] = {
        {"cls", "?GetCls@ElementSkill@GNET@@UBEHXZ", -1},
        {"max_level", "?GetMaxLevel@ElementSkill@GNET@@UAEHXZ", -1},
        {"mp", "?GetMpCost@ElementSkill@GNET@@UAEHXZ", -1},
        {"recarga_ms", "?GetCoolingTime@ElementSkill@GNET@@UAEHXZ", -1},
        {"execucao_ms", "?GetExecuteTime@ElementSkill@GNET@@UAEHXZ", -1},
        {"nivel_exigido", "?GetRequiredLevel@ElementSkill@GNET@@UAEHXZ", -1},
        {"sp_exigido", "?GetRequiredSp@ElementSkill@GNET@@UAEHXZ", -1},
        {"dinheiro_exigido", "?GetRequiredMoney@ElementSkill@GNET@@UAEHXZ", -1},
        {"item_exigido", "?GetRequiredBook@ElementSkill@GNET@@UAEHXZ", -1},
        {"apcost", "?GetApCost@ElementSkill@GNET@@UAEHXZ", -1},
        {"arrowcost", "?GetArrowCost@ElementSkill@GNET@@UAEHXZ", -1},
    };
    const int N = sizeof(v) / sizeof(v[0]);
    int imax = -1;
    for (int k = 0; k < N; k++) {
        v[k].i = indice(vt, v[k].nome);
        fprintf(stderr, "vtable %s = %d%s\n", v[k].chave, v[k].i, v[k].i == -2 ? " (endereço repetido: fora)" : v[k].i == -1 ? " (sem export: fora)" : "");
        if (!strcmp(v[k].chave, "max_level")) imax = v[k].i;
    }
    if (imax < 0 && niveis.empty()) { fprintf(stderr, "sem GetMaxLevel: passe niveis.txt\n"); return 2; }
    static const char* est[][2] = {
        {"s_nivel_exigido", "?GetRequiredLevel@ElementSkill@GNET@@SAHIH@Z"},
        {"s_sp_exigido", "?GetRequiredSp@ElementSkill@GNET@@SAHIH@Z"},
        {"s_dinheiro_exigido", "?GetRequiredMoney@ElementSkill@GNET@@SAHIH@Z"},
        {"s_item_exigido", "?GetRequiredBook@ElementSkill@GNET@@SAHIH@Z"},
        {"s_execucao_ms", "?GetExecuteTime@ElementSkill@GNET@@SAHIH@Z"},
        {"s_reino_exigido", "?GetRequiredRealmLevel@ElementSkill@GNET@@SAHIH@Z"},
    };
    FnIdNivel fest[6];
    for (int e = 0; e < 6; e++) fest[e] = (FnIdNivel)sym(est[e][1]);
    if (init) init();
    int habilidades = 0;
    for (unsigned int id = next(0); id != 0; id = next(id)) {
        int maxl;
        if (imax >= 0) {
            void* s1 = criar(id, 1);
            if (!s1) continue;
            maxl = ((FnInt)(*(void***)s1)[imax])(s1);
            destruir(s1);
        } else {
            auto it = niveis.find(id);
            if (it == niveis.end()) { fprintf(stderr, "sem nível máximo para %u\n", id); continue; }
            maxl = it->second;
        }
        if (maxl < 1 || maxl > 200) maxl = 1;
        habilidades++;
        for (int nivel = 1; nivel <= maxl; nivel++) {
            void* s = criar(id, nivel);
            if (!s) continue;
            void** vts = *(void***)s;
            printf("{\"id\":%u,\"nivel\":%d", id, nivel);
            if (imax < 0) printf(",\"max_level\":%d", maxl);
            for (int k = 0; k < N; k++) {
                if (v[k].i < 0) continue;
                printf(",\"%s\":%d", v[k].chave, ((FnInt)vts[v[k].i])(s));
            }
            for (int e = 0; e < 6; e++)
                if (fest[e]) printf(",\"%s\":%d", est[e][0], fest[e](id, nivel));
            printf("}\n");
            destruir(s);
        }
    }
    fprintf(stderr, "habilidades: %d\n", habilidades);
    return 0;
}
