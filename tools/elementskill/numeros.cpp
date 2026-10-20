// B303: lê os números das habilidades do elementskill.dll instalado no cliente (o binário é o juiz).
// Uso: numeros.exe <caminho do elementskill.dll>  → uma linha JSON por (habilidade, nível) na saída.
// Os índices da tabela virtual são descobertos no próprio binário: a vtable da base (??_7ElementSkill@GNET@@6B@,
// exportada) aponta para as funções da base, também exportadas — o endereço de cada uma dá o índice.
#include <windows.h>
#include <stdio.h>
#include <string.h>

typedef void(__cdecl* FnVoid)();
typedef unsigned int(__cdecl* FnNext)(unsigned int);
typedef void*(__cdecl* FnCreate)(unsigned int, int);
typedef int(__cdecl* FnIdNivel)(unsigned int, int);
typedef int(__thiscall* FnInt)(void*);
typedef void(__thiscall* FnDestroy)(void*);

static HMODULE dll;
static void* sym(const char* n) {
    void* p = (void*)GetProcAddress(dll, n);
    if (!p) { fprintf(stderr, "sem export %s\n", n); }
    return p;
}

// Índice, na vtable da base, da função de base exportada com o nome `n`.
static int indice(void** vt, int tamanho, const char* n) {
    void* f = sym(n);
    if (!f) return -1;
    for (int i = 0; i < tamanho; i++) if (vt[i] == f) return i;
    return -1;
}

int main(int argc, char** argv) {
    if (argc < 2) { fprintf(stderr, "uso: numeros.exe elementskill.dll\n"); return 2; }
    dll = LoadLibraryA(argv[1]);
    if (!dll) { fprintf(stderr, "LoadLibrary falhou: %lu\n", GetLastError()); return 1; }
    FnVoid init = (FnVoid)sym("?InitStaticData@ElementSkill@GNET@@SAXXZ");
    FnNext next = (FnNext)sym("?NextSkill@ElementSkill@GNET@@SAII@Z");
    FnCreate criar = (FnCreate)sym("?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
    FnDestroy destruir = (FnDestroy)sym("?Destroy@ElementSkill@GNET@@QAEXXZ");
    void** vt = (void**)sym("??_7ElementSkill@GNET@@6B@");
    if (!init || !next || !criar || !destruir || !vt) return 1;
    const int TAM = 64;
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
    for (int a = 0; a < TAM; a++) fprintf(stderr, "vt[%d]=%p\n", a, vt[a]);
    for (int k = 0; k < N; k++) {
        v[k].i = indice(vt, TAM, v[k].nome);
        fprintf(stderr, "vtable %s = %d\n", v[k].chave, v[k].i);
    }
    init();
    int imax = -1;
    for (int k = 0; k < N; k++) if (!strcmp(v[k].chave, "max_level")) imax = v[k].i;
    int habilidades = 0;
    for (unsigned int id = next(0); id != 0; id = next(id)) {
        void* s1 = criar(id, 1);
        if (!s1) continue;
        int maxl = ((FnInt)(*(void***)s1)[imax])(s1);
        destruir(s1);
        if (maxl < 1 || maxl > 200) maxl = 1;
        habilidades++;
        for (int nivel = 1; nivel <= maxl; nivel++) {
            void* s = criar(id, nivel);
            if (!s) continue;
            void** vts = *(void***)s;
            printf("{\"id\":%u,\"nivel\":%d", id, nivel);
            for (int k = 0; k < N; k++) {
                if (v[k].i < 0) continue;
                printf(",\"%s\":%d", v[k].chave, ((FnInt)vts[v[k].i])(s));
            }
            static const char* est[][2] = {
                {"s_nivel_exigido", "?GetRequiredLevel@ElementSkill@GNET@@SAHIH@Z"},
                {"s_sp_exigido", "?GetRequiredSp@ElementSkill@GNET@@SAHIH@Z"},
                {"s_dinheiro_exigido", "?GetRequiredMoney@ElementSkill@GNET@@SAHIH@Z"},
                {"s_item_exigido", "?GetRequiredBook@ElementSkill@GNET@@SAHIH@Z"},
                {"s_execucao_ms", "?GetExecuteTime@ElementSkill@GNET@@SAHIH@Z"},
                {"s_reino_exigido", "?GetRequiredRealmLevel@ElementSkill@GNET@@SAHIH@Z"},
            };
            for (int e = 0; e < 6; e++) {
                FnIdNivel f = (FnIdNivel)GetProcAddress(dll, est[e][1]);
                if (f) printf(",\"%s\":%d", est[e][0], f(id, nivel));
            }
            printf("}\n");
            destruir(s);
        }
    }
    fprintf(stderr, "habilidades: %d\n", habilidades);
    return 0;
}
