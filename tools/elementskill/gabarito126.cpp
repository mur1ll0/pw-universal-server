// Fase 7 do DLL de dados (spec 08 §13): o gabarito do elementskill.dll do 1.2.6 — tudo o que o DLL devolve ao
// executável 1.2.6, para toda habilidade e todo nível. O nosso DLL 1.2.6 só vale se este mesmo programa, rodado nele,
// der a mesma saída (tools/elementskill/diferencial.py), exceto a parte "interno".
//
// Uso: gabarito126.exe <elementskill.dll> <formatos.txt> [--interno] > gabarito126.jsonl
//
// O 1.2.6 tem outro contrato (spec 08 §13): a classe concreta que o Create devolve tem 31 slots. Este programa os
// chama pelo slot físico do 1.2.6 e escreve cada valor com o número do slot **lógico** (o do 1.5.5, spec 08 §3), para
// o mesmo gerar_modelo.py e o mesmo habilidades.pwsk servirem às duas versões.
//
// `--interno` (só no DLL original): também os dados do stub que nenhum slot expõe, lidos nos deslocamentos provados
// pela desmontagem do DLL 1.2.6 — o objeto é {vtable, stub*, …} (Create em 100017CC); no stub: +0x08 classe e +0x20
// rank (LearnCondition, 1000101F), +0x18 nível máximo (Create), +0x44 nível do pré-requisito (LearnCondition/PetLearn),
// e a virtual 3 do stub = proficiência exigida no nível (LearnCondition 1000116D, GetAbilityPercent 10001598). A
// comparação entre os dois DLLs não usa esses números: usa as varreduras pela API pública que dependem deles.
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <map>
#include <string>
#include <vector>

typedef unsigned int(__cdecl* FnNext)(unsigned int);
typedef void*(__cdecl* FnCreate)(unsigned int, int);
typedef void(__thiscall* FnDestroy)(void*);
typedef int(__thiscall* FnInt)(void*);
typedef float(__thiscall* FnFloatF)(void*, float);
typedef void*(__thiscall* FnPtr)(void*);
typedef const wchar_t*(__thiscall* FnIntro)(void*, wchar_t*, int, const void*);
typedef bool(__thiscall* FnVW)(void*, int);

static HMODULE dll;
static void* sym(const char* n) { return (void*)GetProcAddress(dll, n); }

struct SondaStr {
    std::map<int, std::wstring>* textos;
    const wchar_t* fixo;
    SondaStr(std::map<int, std::wstring>* t, const wchar_t* f) : textos(t), fixo(f) {}
    virtual wchar_t* Find(int n) const {
        if (fixo) return (wchar_t*)fixo;
        auto it = textos->find(n);
        return it == textos->end() ? (wchar_t*)L"" : (wchar_t*)it->second.c_str();
    }
    virtual ~SondaStr() {}
};

// Estruturas do 1.2.6 pelos deslocamentos lidos nas funções do DLL (spec 08 §13).
struct UseRequirement126 { int mp, ap, form, weapon, freepackage, arrow; };   // Condition 100012F1: +0..+0x14
struct LearnRequirement126 { int level, sp, money, profession, rank; };       // LearnCondition 1000101F: +0..+0x10
struct PetRequirement { int level, sp; int* list; int lsize; };               // PetLearn 10001624: +0..+0xC

static void hex(const unsigned char* p, size_t n) { for (size_t i = 0; i < n; i++) printf("%02x", p[i]); }
static void str_a(const char* s) { printf("\""); if (s) hex((const unsigned char*)s, strlen(s)); printf("\""); }
static void str_w(const wchar_t* s) { printf("\""); if (s) hex((const unsigned char*)s, wcslen(s) * 2); printf("\""); }

static unsigned int semente = 12345;
static int sorteio(int n) { semente = semente * 1103515245u + 12345u; return (int)((semente >> 8) % (unsigned)n); }

#define PROTEGIDO(expr, ok) __try { expr; ok = 1; } __except (EXCEPTION_EXECUTE_HANDLER) { ok = 0; }
static int chamar_int(void* s, int slot, int* ok) { int r = 0; PROTEGIDO(r = ((FnInt)(*(void***)s)[slot])(s), *ok); return r; }
static void* p_ptr(void* s, int slot, int* ok) { void* r = NULL; PROTEGIDO(r = ((FnPtr)(*(void***)s)[slot])(s), *ok); return r; }
static float p_float(void* s, int slot, float a, int* ok) { float r = 0; PROTEGIDO(r = ((FnFloatF)(*(void***)s)[slot])(s, a), *ok); return r; }
static const wchar_t* p_intro(void* s, wchar_t* buf, int len, const void* tabela, int* ok) { const wchar_t* r = NULL; PROTEGIDO(r = ((FnIntro)(*(void***)s)[4])(s, buf, len, tabela), *ok); return r; }
typedef int(__cdecl* FnCond)(unsigned int, void*, int);
static int p_cond(FnCond f, unsigned int id, void* q, int nivel, int* ok) { int r = 0; PROTEGIDO(r = f(id, q, nivel), *ok); return r; }
// A virtual 3 do stub (só com --interno): `stub->f(skill)`, thiscall com o objeto como argumento.
typedef int(__thiscall* FnStub)(void*, void*);
static int p_stub3(void* s, int* ok) { int r = 0; PROTEGIDO(void* st = *(void**)((char*)s + 4); r = ((FnStub)(*(void***)st)[3])(st, s), *ok); return r; }

// Slot lógico (spec 08 §3, numeração do 1.5.5) → slot físico na classe concreta do 1.2.6 (spec 08 §13); -1 = não há.
static const int FISICO[53] = {
    0, 1, 2, 3, 4, -1, 5, 6, 7, 8,          // 0-9: GetName … GetPrayRange (5 GetCls não existe)
    -1, 9, 10, 11, 12, -1, -1, -1, -1, 14,  // 10-19: GetRank não; Required Level/Sp/Book/Money; 15-18 não; 19 GetShowOrder
    15, -1, 16, 17, 18, 19, 20, -1, -1, 21, // 20-29: SetLevel; 21 GetMaxLevel não; IsWarmup…IsDurative; GetRangeType; 27/28 não; GetEffect
    22, 23, 24, 25, 26, -1, -1, -1, -1, -1, // 30-39: GetElseEffect; GetMpCost; GetApCost; GetArrowCost; ValidWeapon
    27, 28, -1, -1, -1, -1, -1, -1, -1, -1, // 40-49: GetAllowForms; Interrupt
    -1, -1, -1};
const int F_REQSKILL = 13, F_JUNIOR = 29;  // só no 1.2.6: GetRequiredSkill() e GetJunior(), int

int main(int argc, char** argv) {
    if (argc < 3) { fprintf(stderr, "uso: gabarito126.exe elementskill.dll formatos.txt [--interno]\n"); return 2; }
    bool interno = argc > 3 && !strcmp(argv[3], "--interno");
    dll = LoadLibraryA(argv[1]);
    if (!dll) return 1;
    std::map<int, std::wstring> textos;
    {
        FILE* f = fopen(argv[2], "r");
        static char linha[1 << 16];
        while (f && fgets(linha, sizeof linha, f)) {
            int id = atoi(linha);
            char* h = strchr(linha, ' ');
            std::wstring w;
            if (h) {
                h++;
                size_t n = strcspn(h, "\r\n");
                for (size_t k = 0; k + 4 <= n; k += 4) {
                    unsigned int a, b;
                    sscanf(h + k, "%2x%2x", &a, &b);
                    w.push_back((wchar_t)(a | (b << 8)));
                }
            }
            textos[id * 10 + 1] = w;
        }
        if (f) fclose(f);
        fprintf(stderr, "formatos: %zu\n", textos.size());
    }
    FnNext next = (FnNext)sym("?NextSkill@ElementSkill@GNET@@SAII@Z");
    FnCreate criar = (FnCreate)sym("?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
    FnDestroy destruir = (FnDestroy)sym("?Destroy@ElementSkill@GNET@@QAEXXZ");
    typedef int(__cdecl* SI)(unsigned int);
    typedef int(__cdecl* SII)(unsigned int, int);
    typedef const char*(__cdecl* SA)(unsigned int);
    typedef const wchar_t*(__cdecl* SW)(unsigned int);
    struct { const char* nome; const char* sym; char tipo; } est[] = {
        {"GetType", "?GetType@ElementSkill@GNET@@SADI@Z", 'c'},
        {"GetIcon", "?GetIcon@ElementSkill@GNET@@SAPBDI@Z", 'a'},
        {"GetName", "?GetName@ElementSkill@GNET@@SAPBGI@Z", 'w'},
        {"GetNativeName", "?GetNativeName@ElementSkill@GNET@@SAPBDI@Z", 'a'},
        {"GetEffect", "?GetEffect@ElementSkill@GNET@@SAPBDI@Z", 'a'},
    };
    struct { const char* nome; const char* sym; } est_nivel[] = {
        {"GetRequiredLevel", "?GetRequiredLevel@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredSp", "?GetRequiredSp@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredMoney", "?GetRequiredMoney@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredBook", "?GetRequiredBook@ElementSkill@GNET@@SAHIH@Z"},
        {"GetExecuteTime", "?GetExecuteTime@ElementSkill@GNET@@SAHIH@Z"},
    };
    FnCond learn = (FnCond)sym("?LearnCondition@ElementSkill@GNET@@SAHIAAULearnRequirement@2@H@Z");
    FnCond cond = (FnCond)sym("?Condition@ElementSkill@GNET@@SAHIAAUUseRequirement@2@H@Z");
    FnCond pet = (FnCond)sym("?PetLearn@ElementSkill@GNET@@SAHIAAUPetRequirement@2@H@Z");
    typedef int(__cdecl* FnSet)(unsigned int, int);
    FnSet set_ability = (FnSet)sym("?SetAbility@ElementSkill@GNET@@SAHIH@Z");
    FnSet set_level = (FnSet)sym("?SetLevel@ElementSkill@GNET@@SAHIH@Z");
    typedef int(__cdecl* FnAb)(unsigned int);
    FnAb ability = (FnAb)sym("?GetAbility@ElementSkill@GNET@@SAHI@Z");
    FnAb ability_pct = (FnAb)sym("?GetAbilityPercent@ElementSkill@GNET@@SAHI@Z");
    typedef void(__cdecl* FnLoad)(void*);
    FnLoad load = (FnLoad)sym("?LoadSkillData@ElementSkill@GNET@@SAXPAX@Z");
    if (!next || !criar || !destruir || !learn || !cond || !pet || !set_ability || !set_level || !ability || !ability_pct || !load) {
        fprintf(stderr, "faltam exports do 1.2.6\n");
        return 1;
    }
    static unsigned char vazio[4] = {0, 0, 0, 0};
    load(vazio);  // mapa do jogador limpo (LoadSkillData limpa antes de carregar: 1000452B)

    const int slots_int[] = {5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 19, 21, 26, 27, 31, 32, 33, 42, 45, 46, 47, 48, 49, 50};
    const int slots_byte[] = {2, 22, 23, 24, 25, 35, 36, 37, 38, 39, 40, 41, 52};
    const int slots_a[] = {1, 3, 29, 30};
    const float praia[][2] = {{0, 0}, {1, 0}, {5, 0}, {0, 1}, {3.5f, 2}, {10, 0.5f}};

    SondaStr sonda_bruta(&textos, L"%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x");
    SondaStr sonda_real(&textos, NULL);
    static wchar_t buf[8192];
    int habilidades = 0;
    for (unsigned int id = next(0); id; id = next(id)) {
        void* s1 = criar(id, 1);
        if (!s1) continue;
        int ok;
        // Nível máximo pela API: o maior nível em que o LearnCondition não devolve 3 (nível fora da faixa), com o mapa
        // do jogador vazio (sem o 11 da sobrescrita) e tudo o mais de sobra.
        load(vazio);
        int maxl = 0;
        for (int l = 1; l <= 200; l++) {
            LearnRequirement126 q = {9999, 0x7fffffff, 0x7fffffff, 0, 9999};
            if (p_cond(learn, id, &q, l, &ok) != 3 && ok) maxl = l;
        }
        if (maxl < 1) maxl = 1;
        habilidades++;
        printf("{\"habilidade\":%u,\"max_level\":%d,\"estaticas\":{", id, maxl);
        for (int e = 0; e < (int)(sizeof est / sizeof est[0]); e++) {
            void* f = sym(est[e].sym);
            printf("%s\"%s\":", e ? "," : "", est[e].nome);
            if (!f) { printf("null"); continue; }
            switch (est[e].tipo) {
                case 'c': printf("%d", (int)(signed char)((SI)f)(id)); break;
                case 'a': str_a(((SA)f)(id)); break;
                case 'w': str_w(((SW)f)(id)); break;
            }
        }
        printf("}");
        {
            FnVW vw = (FnVW)(*(void***)s1)[FISICO[34]];
            std::vector<int> sim;
            int falsos = 0;
            for (int w = 0; w < 65536; w++) { if (vw(s1, w)) sim.push_back(w); else falsos++; }
            printf(",\"armas\":");
            if (!falsos) printf("\"todas\"");
            else { printf("["); for (size_t k = 0; k < sim.size(); k++) printf("%s%d", k ? "," : "", sim[k]); printf("]"); }
        }
        // Classe e rank pela API: LearnCondition no nível 1, profissão 0..15 × rank 0..60 (um caractere por código).
        printf(",\"classe_rank\":\"");
        for (int prof = 0; prof < 16; prof++) {
            for (int rank = 0; rank <= 60; rank++) {
                LearnRequirement126 q = {9999, 0x7fffffff, 0x7fffffff, prof, rank};
                int r = p_cond(learn, id, &q, 1, &ok);
                printf("%c", ok ? (r >= 0 && r < 36 ? "0123456789abcdefghijklmnopqrstuvwxyz"[r] : '?') : '!');
            }
            printf(prof < 15 ? "|" : "");
        }
        printf("\"");
        // GetRequiredSkill / GetJunior (int no 1.2.6).
        printf(",\"pre_126\":%d,\"jun_126\":%d", chamar_int(s1, F_REQSKILL, &ok), chamar_int(s1, F_JUNIOR, &ok));
        if (interno) {
            char* st = *(char**)((char*)s1 + 4);
            printf(",\"interno\":{\"max\":%d,\"cls\":%d,\"rank\":%d,\"pre_nivel\":%d}", *(int*)(st + 0x18), *(int*)(st + 8),
                   *(int*)(st + 0x20), *(int*)(st + 0x44));
        }
        printf("}\n");
        destruir(s1);
        for (int nivel = 1; nivel <= maxl; nivel++) {
            void* s = criar(id, nivel);
            if (!s) { printf("{\"id\":%u,\"nivel\":%d,\"criar\":null}\n", id, nivel); continue; }
            printf("{\"id\":%u,\"nivel\":%d,\"v\":{", id, nivel);
            int primeiro = 1;
            for (int sl : slots_int) {
                printf("%s\"%d\":", primeiro ? "" : ",", sl);
                primeiro = 0;
                if (FISICO[sl] < 0) { printf("0"); continue; }
                int r = chamar_int(s, FISICO[sl], &ok);
                if (ok) printf("%d", r); else printf("\"erro\"");
            }
            for (int sl : slots_byte) {
                printf(",\"%d\":", sl);
                if (FISICO[sl] < 0) { printf("0"); continue; }
                int r = chamar_int(s, FISICO[sl], &ok);
                if (ok) printf("%d", (int)(signed char)(r & 0xff)); else printf("\"erro\"");
            }
            for (int sl : slots_a) { const char* r = (const char*)p_ptr(s, FISICO[sl], &ok); printf(",\"%d\":", sl); if (ok) str_a(r); else printf("\"erro\""); }
            { const wchar_t* r = (const wchar_t*)p_ptr(s, 0, &ok); printf(",\"0\":"); if (ok) str_w(r); else printf("\"erro\""); }
            printf("}");
            // GetPrayRange(float alcance) — um argumento no 1.2.6 (o bônus da sonda é ignorado).
            printf(",\"alcance\":[");
            for (int k = 0; k < 6; k++) { float r = p_float(s, FISICO[9], praia[k][0], &ok); printf("%s%.9g", k ? "," : "", ok ? r : -9999.0f); }
            printf("]");
            printf(",\"genio\":null,\"pre\":[],\"pre_lista\":[],\"juniores\":[],\"juniores_lista\":[]");
            for (int ab = 0; ab < 2; ab++) {
                set_ability(id, ab ? 500 : 0);
                const wchar_t* r = NULL;
                buf[0] = 0;
                r = p_intro(s, buf, 8192, &sonda_bruta, &ok);
                printf(",\"intro_bruta%s\":", ab ? "_500" : "");
                if (ok && r) str_w(r); else printf("null");
                buf[0] = 0;
                r = p_intro(s, buf, 8192, &sonda_real, &ok);
                printf(",\"intro%s\":", ab ? "_500" : "");
                if (ok && r) str_w(r); else printf("null");
                printf(",\"pct%s\":%d", ab ? "_500" : "", ability_pct(id));
            }
            set_ability(id, 0);
            // Proficiência por nível pela API: GetAbilityPercent com o nível aprendido = este nível.
            printf(",\"pct_nivel\":[");
            const int abs_[] = {0, 1, 7, 50, 100, 333, 1000, 4096, 10000, 65535};
            set_level(id, nivel);
            for (int k = 0; k < 10; k++) { set_ability(id, abs_[k]); printf("%s%d", k ? "," : "", ability_pct(id)); }
            printf("]");
            set_level(id, 1);
            set_ability(id, 0);
            printf(",\"est\":{");
            for (int e = 0; e < (int)(sizeof est_nivel / sizeof est_nivel[0]); e++) {
                SII f = (SII)sym(est_nivel[e].sym);
                printf("%s\"%s\":", e ? "," : "", est_nivel[e].nome);
                if (f) printf("%d", f(id, nivel)); else printf("null");
            }
            printf(",\"GetRequiredRealmLevel\":null,\"GetMaxAbility\":null}");
            if (interno) { int r = p_stub3(s, &ok); printf(",\"interno_prof\":%d", ok ? r : 0); }
            // Condições com sondas fixas; o LearnCondition com proficiência sorteada antes de cada sonda.
            printf(",\"aprender\":\"");
            for (int k = 0; k < 24; k++) {
                set_ability(id, sorteio(3) ? sorteio(20000) : 0);
                LearnRequirement126 q = {sorteio(160), sorteio(3) ? sorteio(2000000) : 99999999, sorteio(3) ? sorteio(3000000) : 99999999, sorteio(13), sorteio(40)};
                int r = p_cond(learn, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            set_ability(id, 0);
            printf("\",\"usar\":\"");
            for (int k = 0; k < 24; k++) {
                UseRequirement126 q = {sorteio(4) ? sorteio(5000) : 99999, sorteio(400), sorteio(9), sorteio(3) ? 0 : sorteio(50000), sorteio(2) * 10, sorteio(2) ? 0 : sorteio(200)};
                int r = p_cond(cond, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            printf("\",\"mascote\":\"");
            static int lista_pet[8];
            for (int k = 0; k < 12; k++) {
                for (int j = 0; j < 8; j++) lista_pet[j] = sorteio(4000);
                if (sorteio(2)) { lista_pet[2 * sorteio(4)] = chamar_int(s, F_REQSKILL, &ok); }
                PetRequirement q = {sorteio(160), sorteio(200000), lista_pet, sorteio(9)};
                int r = p_cond(pet, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            printf("\"}\n");
            destruir(s);
        }
    }
    // Globais. O mapa do jogador: LoadSkillData com 3 habilidades e o que GetAbility/GetAbilityPercent devolvem; e a
    // sobrescrita (11 no LearnCondition) de quem tem a habilidade aprendida como júnior.
    typedef int(__cdecl* FnVer)();
    FnVer ver = (FnVer)sym("?GetVersion@ElementSkill@GNET@@SAHXZ");
    printf("{\"versao\":%d}\n", ver ? ver() : -1);
    {
        std::vector<unsigned int> ids;
        for (unsigned int id = next(0); id; id = next(id)) ids.push_back(id);
        unsigned char pacote[4 + 5 * 3];
        unsigned int n = 3;
        memcpy(pacote, &n, 4);
        for (int k = 0; k < 3; k++) {
            short i = (short)ids[k * ids.size() / 3], ab = (short)(100 * k + 37);
            signed char lv = (signed char)(k + 1);
            memcpy(pacote + 4 + 5 * k, &i, 2);
            pacote[4 + 5 * k + 2] = (unsigned char)lv;
            memcpy(pacote + 4 + 5 * k + 3, &ab, 2);
        }
        load(pacote);
        printf("{\"carga\":[");
        for (int k = 0; k < 3; k++) { unsigned int i = ids[k * ids.size() / 3]; printf("%s[%u,%d,%d]", k ? "," : "", i, ability(i), ability_pct(i)); }
        printf("]}\n");
        // Sobrescrita: para cada habilidade com júnior, aprende só ela e pergunta o LearnCondition da júnior.
        printf("{\"sobrescrita\":[");
        int pa = 1;
        for (size_t k = 0; k < ids.size(); k++) {
            void* s = criar(ids[k], 1);
            int ok;
            int jun = s ? chamar_int(s, F_JUNIOR, &ok) : 0;
            if (s) destruir(s);
            if (!jun) continue;
            load(vazio);
            set_level(ids[k], 1);
            LearnRequirement126 q = {9999, 0x7fffffff, 0x7fffffff, 0, 0};
            printf("%s[%u,%d,%d]", pa ? "" : ",", ids[k], jun, p_cond(learn, (unsigned)jun, &q, 1, &ok));
            pa = 0;
        }
        printf("]}\n");
        load(vazio);
    }
    typedef void*(__cdecl* FnQV)(int, int);
    typedef void*(__cdecl* FnQT)(int);
    FnQV qv = (FnQV)sym("?Query@VisibleState@GNET@@SAPBV12@HH@Z");
    FnQT qt = (FnQT)sym("?Query@TeamState@GNET@@SAPBV12@H@Z");
    for (int prof = 0; qv && prof < 16; prof++)
        for (int i = 0; i < 4096; i++) {
            void* e = qv(prof, i);
            if (!e) continue;
            void** v = *(void***)e;
            printf("{\"estado_visivel\":[%d,%d],\"id\":%d,\"nome\":", prof, i, ((FnInt)v[0])(e));
            str_w((const wchar_t*)((FnPtr)v[1])(e)); printf(",\"hh\":"); str_a((const char*)((FnPtr)v[2])(e));
            printf(",\"efeito\":"); str_a((const char*)((FnPtr)v[3])(e)); printf("}\n");
        }
    for (int i = 0; qt && i < 4096; i++) {
        void* e = qt(i);
        if (!e) continue;
        void** v = *(void***)e;
        printf("{\"estado_equipe\":%d,\"id\":%d,\"nome\":", i, ((FnInt)v[0])(e));
        str_w((const wchar_t*)((FnPtr)v[1])(e)); printf(",\"icone\":"); str_a((const char*)((FnPtr)v[2])(e)); printf("}\n");
    }
    fprintf(stderr, "habilidades: %d\n", habilidades);
    return 0;
}
