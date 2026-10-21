// B314 (fase 2 do DLL de dados): o gabarito — tudo o que o elementskill.dll original devolve ao executável, para toda
// habilidade e todo nível, pelo contrato da spec 08. O DLL de dados (fase 3) só vale se este mesmo programa, rodado
// nele, der saída idêntica.
//
// Uso: gabarito.exe <elementskill.dll> <formatos.txt> > gabarito.jsonl
//   formatos.txt: linhas "<id> <hex UTF-16LE do texto N×10+1 do skillstr.txt>" (tools/elementskill/formatos.py).
// Saída (JSON por linha):
//   {"habilidade":id, "estaticas":{…}, "armas":…, "vida":…}            uma por habilidade
//   {"id":id, "nivel":n, "v":{slot:valor…}, "intro_bruta":[…], "intro":"…", …}   uma por (habilidade, nível)
//   {"estado_visivel":…} / {"estado_equipe":…} / {"inerentes":…} / {"versao":…}   globais
// Strings `char*` (GBK) saem em hex, para comparar byte a byte; `wchar_t*` em hex UTF-16LE.
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <map>
#include <string>
#include <vector>

typedef void(__cdecl* FnVoid)();
typedef unsigned int(__cdecl* FnNext)(unsigned int);
typedef void*(__cdecl* FnCreate)(unsigned int, int);
typedef void(__thiscall* FnDestroy)(void*);
typedef int(__thiscall* FnInt)(void*);
typedef int(__thiscall* FnIntI)(void*, int);
typedef int(__thiscall* FnIntII)(void*, int, int);
typedef float(__thiscall* FnFloatFF)(void*, float, float);
typedef void*(__thiscall* FnPtr)(void*);
typedef void*(__thiscall* FnPtrI)(void*, int);
typedef const wchar_t*(__thiscall* FnIntro)(void*, wchar_t*, int, const void*);
typedef void(__thiscall* FnArr)(void*, void*);

static HMODULE dll;
static void* sym(const char* n) { return (void*)GetProcAddress(dll, n); }

// ---- interfaces que o executável implementa (spec 08 §4) ----
struct SondaStr {
    std::map<int, std::wstring>* textos;
    const wchar_t* fixo;  // se não nulo, devolvido por Find no lugar do texto
    SondaStr(std::map<int, std::wstring>* t, const wchar_t* f) : textos(t), fixo(f) {}
    virtual wchar_t* Find(int n) const {
        if (fixo) return (wchar_t*)fixo;
        auto it = textos->find(n);
        return it == textos->end() ? (wchar_t*)L"" : (wchar_t*)it->second.c_str();
    }
    virtual ~SondaStr() {}
};
// `skill_array_interface<T>` (spec 08 §4): o DLL chama [+4] assign, [+8] clear e [+0x10] push_back
// (`GetComboSkActivated`, desmontagem em 10002410); [+0] e [+0xC] não são usadas.
template <class T> struct Lista {
    std::vector<T> v;
    virtual void nada0() {}
    virtual void assign(const T* a, const T* b) { v.assign(a, b); }
    virtual void clear() { v.clear(); }
    virtual void nada3() {}
    virtual void push_back(const T& x) { v.push_back(x); }
};
struct Par { unsigned int id; int nivel; };

// ---- estruturas (spec 08 §5) ----
struct ComboSkillState { unsigned int skillid; int arg[3]; };
struct UseRequirement { int mp, ap, form, weapon, freepackage, arrow, move_env; bool is_combat; int hp, max_hp; ComboSkillState combo; };
struct LearnRequirement { int level, sp, money, profession, rank, realm_level; };
struct PetRequirement { int level, sp; int* list; int lsize; };
struct GoblinRequirement { int genius[5]; int profession, sp, money, level, mp; };
struct GoblinUseRequirement { int mp, ap; int genius[5]; int profession, level, move_env; };

static void hex(const unsigned char* p, size_t n) { for (size_t i = 0; i < n; i++) printf("%02x", p[i]); }
static void str_a(const char* s) { printf("\""); if (s) hex((const unsigned char*)s, strlen(s)); printf("\""); }
static void str_w(const wchar_t* s) { printf("\""); if (s) hex((const unsigned char*)s, wcslen(s) * 2); printf("\""); }

// Gerador fixo (as sondas das condições são as mesmas em qualquer rodada).
static unsigned int semente = 12345;
static int sorteio(int n) { semente = semente * 1103515245u + 12345u; return (int)((semente >> 8) % (unsigned)n); }

// Chamada protegida: um método que cai vira "erro" no gabarito (o DLL de dados tem de cair igual ou não cair —
// registrado para a comparação).
#define PROTEGIDO(expr, ok) __try { expr; ok = 1; } __except (EXCEPTION_EXECUTE_HANDLER) { ok = 0; }

static int chamar_int(void* s, int slot, int* ok) { int r = 0; PROTEGIDO(r = ((FnInt)(*(void***)s)[slot])(s), *ok); return r; }
// Chamadas protegidas fora do `main` (o `__try` não convive com objetos que têm destrutor).
static void* p_ptr(void* s, int slot, int* ok) { void* r = NULL; PROTEGIDO(r = ((FnPtr)(*(void***)s)[slot])(s), *ok); return r; }
static void* p_ptr_i(void* s, int slot, int a, int* ok) { void* r = NULL; PROTEGIDO(r = ((FnPtrI)(*(void***)s)[slot])(s, a), *ok); return r; }
static float p_float(void* s, int slot, float a, float b, int* ok) { float r = 0; PROTEGIDO(r = ((FnFloatFF)(*(void***)s)[slot])(s, a, b), *ok); return r; }
static const wchar_t* p_intro(void* s, wchar_t* buf, int len, const void* tabela, int* ok) { const wchar_t* r = NULL; PROTEGIDO(r = ((FnIntro)(*(void***)s)[4])(s, buf, len, tabela), *ok); return r; }
static void p_arr(void* s, int slot, void* lista, int* ok) { PROTEGIDO(((FnArr)(*(void***)s)[slot])(s, lista), *ok); }
typedef int(__cdecl* FnCond)(unsigned int, void*, int);
static int p_cond(FnCond f, unsigned int id, void* q, int nivel, int* ok) { int r = 0; PROTEGIDO(r = f(id, q, nivel), *ok); return r; }
typedef void(__cdecl* FnCombo)(const void*, void*);
static void p_combo(FnCombo f, const void* c, void* l, int* ok) { PROTEGIDO(f(c, l), *ok); }

int main(int argc, char** argv) {
    if (argc < 3) { fprintf(stderr, "uso: gabarito.exe elementskill.dll formatos.txt\n"); return 2; }
    dll = LoadLibraryA(argv[1]);
    if (!dll) return 1;
    // Formatos reais da descrição.
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
    FnVoid init = (FnVoid)sym("?InitStaticData@ElementSkill@GNET@@SAXXZ");
    FnNext next = (FnNext)sym("?NextSkill@ElementSkill@GNET@@SAII@Z");
    FnCreate criar = (FnCreate)sym("?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
    FnDestroy destruir = (FnDestroy)sym("?Destroy@ElementSkill@GNET@@QAEXXZ");
    if (init) init();

    // Estáticas (spec 08 §2).
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
        {"IsGoblinSkill", "?IsGoblinSkill@ElementSkill@GNET@@SA_NI@Z", 'b'},
        {"IsMovingSkill", "?IsMovingSkill@ElementSkill@GNET@@SA_NI@Z", 'b'},
        {"IsOverridden", "?IsOverridden@ElementSkill@GNET@@SA_NI@Z", 'b'},
        {"GetComboSkPreSkill", "?GetComboSkPreSkill@ElementSkill@GNET@@SAHI@Z", 'i'},
        {"GetCommonCoolDown", "?GetCommonCoolDown@ElementSkill@GNET@@SAHI@Z", 'i'},
    };
    struct { const char* nome; const char* sym; } est_nivel[] = {
        {"GetRequiredLevel", "?GetRequiredLevel@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredSp", "?GetRequiredSp@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredMoney", "?GetRequiredMoney@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredBook", "?GetRequiredBook@ElementSkill@GNET@@SAHIH@Z"},
        {"GetRequiredRealmLevel", "?GetRequiredRealmLevel@ElementSkill@GNET@@SAHIH@Z"},
        {"GetExecuteTime", "?GetExecuteTime@ElementSkill@GNET@@SAHIH@Z"},
        {"GetMaxAbility", "?GetMaxAbility@ElementSkill@GNET@@SAHIH@Z"},
    };
    FnCond learn = (FnCond)sym("?LearnCondition@ElementSkill@GNET@@SAHIAAULearnRequirement@2@H@Z");
    FnCond cond = (FnCond)sym("?Condition@ElementSkill@GNET@@SAHIAAUUseRequirement@2@H@Z");
    FnCond pet = (FnCond)sym("?PetLearn@ElementSkill@GNET@@SAHIAAUPetRequirement@2@H@Z");
    FnCond gcond = (FnCond)sym("?GoblinCondition@ElementSkill@GNET@@SAHIAAUGoblinUseRequirement@2@H@Z");
    FnCond glearn = (FnCond)sym("?GoblinLearn@ElementSkill@GNET@@SAHIAAUGoblinRequirement@2@H@Z");
    typedef int(__cdecl* FnSetAb)(unsigned int, int);
    FnSetAb set_ability = (FnSetAb)sym("?SetAbility@ElementSkill@GNET@@SAHIH@Z");
    typedef int(__cdecl* FnAbPct)(unsigned int);
    FnAbPct ability_pct = (FnAbPct)sym("?GetAbilityPercent@ElementSkill@GNET@@SAHI@Z");

    // Slots sem argumento da vtable (spec 08 §3) e o tipo do retorno: i = int, b = bool, c = char.
    const int slots_int[] = {5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 19, 21, 26, 27, 31, 32, 33, 42, 45, 46, 47, 48, 49, 50};
    const int slots_byte[] = {2, 22, 23, 24, 25, 35, 36, 37, 38, 39, 40, 41, 52};
    const int slots_a[] = {1, 3, 29, 30};  // char*
    const float praia[][2] = {{0, 0}, {1, 0}, {5, 0}, {0, 1}, {3.5f, 2}, {10, 0.5f}};

    SondaStr sonda_bruta(&textos, L"%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x|%08x");
    SondaStr sonda_real(&textos, NULL);
    static wchar_t buf[8192];
    int habilidades = 0;
    for (unsigned int id = next(0); id; id = next(id)) {
        void* s1 = criar(id, 1);
        if (!s1) continue;
        int ok;
        int maxl = chamar_int(s1, 21, &ok);
        if (!ok || maxl < 1 || maxl > 200) maxl = 1;
        habilidades++;
        // ---- por habilidade ----
        printf("{\"habilidade\":%u,\"max_level\":%d,\"estaticas\":{", id, maxl);
        for (int e = 0; e < (int)(sizeof est / sizeof est[0]); e++) {
            void* f = sym(est[e].sym);
            printf("%s\"%s\":", e ? "," : "", est[e].nome);
            if (!f) { printf("null"); continue; }
            switch (est[e].tipo) {
                case 'c': printf("%d", (int)(signed char)((SI)f)(id)); break;
                case 'b': printf("%d", ((SI)f)(id) & 0xff); break;
                case 'i': printf("%d", ((SI)f)(id)); break;
                case 'a': str_a(((SA)f)(id)); break;
                case 'w': str_w(((SW)f)(id)); break;
            }
        }
        printf("}");
        // ValidWeapon (slot 34) em todos os ids de 0 a 65535 (não depende do nível).
        {
            typedef bool(__thiscall* FnVW)(void*, int);
            FnVW vw = (FnVW)(*(void***)s1)[34];
            std::vector<int> sim;
            int falsos = 0;
            for (int w = 0; w < 65536; w++) { if (vw(s1, w)) sim.push_back(w); else falsos++; }
            printf(",\"armas\":");
            if (!falsos) printf("\"todas\"");
            else { printf("["); for (size_t k = 0; k < sim.size(); k++) printf("%s%d", k ? "," : "", sim[k]); printf("]"); }
        }
        // CheckHpCondition (slot 51): vida 0..100 de 100 e 0..1000 de 1000 em passos de 7.
        {
            FnIntII hp = (FnIntII)(*(void***)s1)[51];
            printf(",\"vida\":\"");
            for (int v = 0; v <= 100; v++) printf("%d", hp(s1, v, 100) & 1);
            printf("|");
            for (int v = 0; v <= 1000; v += 7) printf("%d", hp(s1, v, 1000) & 1);
            printf("\"");
        }
        printf("}\n");
        destruir(s1);
        // ---- por nível ----
        for (int nivel = 1; nivel <= maxl; nivel++) {
            void* s = criar(id, nivel);
            if (!s) { printf("{\"id\":%u,\"nivel\":%d,\"criar\":null}\n", id, nivel); continue; }
            void** vt = *(void***)s;
            printf("{\"id\":%u,\"nivel\":%d,\"v\":{", id, nivel);
            int primeiro = 1;
            for (int sl : slots_int) { int r = chamar_int(s, sl, &ok); printf("%s\"%d\":%s", primeiro ? "" : ",", sl, ok ? "" : "\"erro\""); if (ok) printf("%d", r); primeiro = 0; }
            for (int sl : slots_byte) { int r = chamar_int(s, sl, &ok); printf(",\"%d\":", sl); if (ok) printf("%d", (int)(signed char)(r & 0xff)); else printf("\"erro\""); }
            for (int sl : slots_a) { const char* r = (const char*)p_ptr(s, sl, &ok); printf(",\"%d\":", sl); if (ok) str_a(r); else printf("\"erro\""); }
            { const wchar_t* r = (const wchar_t*)p_ptr(s, 0, &ok); printf(",\"0\":"); if (ok) str_w(r); else printf("\"erro\""); }
            printf("}");
            // GetPrayRange (slot 9) com pares (alcance da arma, bônus).
            printf(",\"alcance\":[");
            for (int k = 0; k < 6; k++) { float r = p_float(s, 9, praia[k][0], praia[k][1], &ok); printf("%s%.9g", k ? "," : "", ok ? r : -9999.0f); }
            printf("]");
            // GetRequiredGenius (slot 28): 5 int ou null.
            { int* g = (int*)p_ptr_i(s, 28, (int)id, &ok); printf(",\"genio\":"); if (ok && g) printf("[%d,%d,%d,%d,%d]", g[0], g[1], g[2], g[3], g[4]); else printf("null"); }
            // Listas: pré-requisitos (16/17) e juniores (42/43).
            for (int par = 0; par < 2; par++) {
                int sc = par ? 42 : 16, sa = par ? 43 : 17;
                int n = chamar_int(s, sc, &ok);
                printf(",\"%s\":[", par ? "juniores" : "pre");
                for (int k = 0; ok && k < n && k < 64; k++) { Par* p = (Par*)((FnPtrI)vt[sa])(s, k); printf("%s[%u,%d]", k ? "," : "", p->id, p->nivel); }
                printf("]");
                // E pela interface (18/44), que tem de dar o mesmo.
                Lista<Par> l;
                p_arr(s, par ? 44 : 18, &l, &ok);
                printf(",\"%s_lista\":[", par ? "juniores" : "pre");
                for (size_t k = 0; ok && k < l.v.size(); k++) printf("%s[%u,%d]", k ? "," : "", l.v[k].id, l.v[k].nivel);
                printf("]");
            }
            // Descrição: os argumentos crus (24 palavras) e o texto com o formato real; com proficiência 0 e 500.
            for (int ab = 0; ab < 2; ab++) {
                if (set_ability) set_ability(id, ab ? 500 : 0);
                const wchar_t* r = NULL;
                buf[0] = 0;
                r = p_intro(s, buf, 8192, &sonda_bruta, &ok);
                printf(",\"intro_bruta%s\":", ab ? "_500" : "");
                if (ok && r) str_w(r); else printf("null");
                buf[0] = 0;
                r = p_intro(s, buf, 8192, &sonda_real, &ok);
                printf(",\"intro%s\":", ab ? "_500" : "");
                if (ok && r) str_w(r); else printf("null");
                if (ability_pct) printf(",\"pct%s\":%d", ab ? "_500" : "", ability_pct(id));
            }
            if (set_ability) set_ability(id, 0);
            // Estáticas por nível.
            printf(",\"est\":{");
            for (int e = 0; e < (int)(sizeof est_nivel / sizeof est_nivel[0]); e++) {
                SII f = (SII)sym(est_nivel[e].sym);
                printf("%s\"%s\":", e ? "," : "", est_nivel[e].nome);
                if (f) printf("%d", f(id, nivel)); else printf("null");
            }
            printf("}");
            // Condições, com 24 sondas fixas cada (os mesmos sorteios em qualquer rodada).
            printf(",\"aprender\":\"");
            for (int k = 0; learn && k < 24; k++) {
                LearnRequirement q = {sorteio(160), sorteio(3) ? sorteio(2000000) : 99999999, sorteio(3) ? sorteio(3000000) : 99999999, sorteio(13), sorteio(40), sorteio(10)};
                int r = p_cond(learn, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            printf("\",\"usar\":\"");
            for (int k = 0; cond && k < 24; k++) {
                UseRequirement q = {sorteio(4) ? sorteio(5000) : 99999, sorteio(400), sorteio(8), sorteio(3) ? 0 : sorteio(50000), sorteio(2) * 10, sorteio(2) ? 0 : sorteio(200), sorteio(3), (bool)(sorteio(2) != 0), sorteio(10000), 10000, {sorteio(2) ? 0 : id, {sorteio(3), sorteio(3), sorteio(3)}}};
                int r = p_cond(cond, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            printf("\",\"mascote\":\"");
            static int lista_pet[8];
            for (int k = 0; pet && k < 12; k++) {
                for (int j = 0; j < 8; j++) lista_pet[j] = sorteio(4000);
                PetRequirement q = {sorteio(160), sorteio(200000), lista_pet, sorteio(9)};
                int r = p_cond(pet, id, &q, nivel, &ok); printf("%s%d", k ? "," : "", ok ? r : -999);
            }
            printf("\",\"duende\":\"");
            for (int k = 0; glearn && gcond && k < 12; k++) {
                GoblinRequirement q = {{sorteio(30), sorteio(30), sorteio(30), sorteio(30), sorteio(30)}, sorteio(13), sorteio(200000), sorteio(3000000), sorteio(160), sorteio(5000)};
                GoblinUseRequirement u = {sorteio(5000), sorteio(400), {sorteio(30), sorteio(30), sorteio(30), sorteio(30), sorteio(30)}, sorteio(13), sorteio(160), sorteio(3)};
                int ok2;
                int r1 = p_cond(glearn, id, &q, nivel, &ok), r2 = p_cond(gcond, id, &u, nivel, &ok2);
                printf("%s%d/%d", k ? "," : "", ok ? r1 : -999, ok2 ? r2 : -999);
            }
            printf("\"}\n");
            destruir(s);
        }
    }
    // ---- globais ----
    typedef int(__cdecl* FnVer)();
    FnVer ver = (FnVer)sym("?GetVersion@ElementSkill@GNET@@SAHXZ");
    printf("{\"versao\":%d}\n", ver ? ver() : -1);
    typedef void(__cdecl* FnInh)(int, void*);
    FnInh inh = (FnInh)sym("?GetInherentSkills@ElementSkill@GNET@@SAXHAAV?$skill_array_interface@I@2@@Z");
    for (int cls = 0; inh && cls < 64; cls++) {
        Lista<unsigned int> l;
        inh(cls, &l);
        if (l.v.empty()) continue;
        printf("{\"inerentes\":%d,\"lista\":[", cls);
        for (size_t k = 0; k < l.v.size(); k++) printf("%s%u", k ? "," : "", l.v[k]);
        printf("]}\n");
    }
    FnCombo combo = (FnCombo)sym("?GetComboSkActivated@ElementSkill@GNET@@SAXABUComboSkillState@2@AAV?$skill_array_interface@U?$pair@IH@_STL@@@2@@Z");
    for (unsigned int id = next(0); combo && id; id = next(id)) {
        for (int a = 0; a < 4; a++) {
            ComboSkillState c = {id, {a, a == 3 ? 1 : 0, 0}};
            Lista<Par> l;
            int ok;
            p_combo(combo, &c, &l, &ok);
            if (!ok) { printf("{\"combo\":%u,\"arg\":%d,\"erro\":true}\n", id, a); continue; }
            if (l.v.empty()) continue;
            printf("{\"combo\":%u,\"arg\":%d,\"lista\":[", id, a);
            for (size_t k = 0; k < l.v.size(); k++) printf("%s[%u,%d]", k ? "," : "", l.v[k].id, l.v[k].nivel);
            printf("]}\n");
        }
    }
    // B317: a condição de combo, varrida pela API pública (vale igual nos dois DLLs): para cada pré-combo, cada
    // argumento de −1000 a 1000 (os outros 0); as faixas em que cada habilidade seguinte aparece e o seu intervalo.
    {
        typedef int(__cdecl* FnPre)(unsigned int);
        FnPre pre = (FnPre)sym("?GetComboSkPreSkill@ElementSkill@GNET@@SAHI@Z");
        std::map<int, int> pres;
        for (unsigned int id = next(0); pre && id; id = next(id)) if (pre(id)) pres[pre(id)] = 1;
        for (auto& pr : pres) {
            for (int a = 0; combo && a < 3; a++) {
                std::map<unsigned int, std::vector<int> > presente;
                std::map<unsigned int, int> intervalo;
                for (int v = -1000; v <= 1000; v++) {
                    ComboSkillState c = {(unsigned int)pr.first, {0, 0, 0}};
                    c.arg[a] = v;
                    Lista<Par> l;
                    int ok;
                    p_combo(combo, &c, &l, &ok);
                    for (size_t k = 0; ok && k < l.v.size(); k++) { presente[l.v[k].id].push_back(v); intervalo[l.v[k].id] = l.v[k].nivel; }
                }
                printf("{\"combo_varredura\":%d,\"arg\":%d,\"ativas\":{", pr.first, a);
                int pa = 1;
                for (auto& e : presente) {
                    printf("%s\"%u\":[", pa ? "" : ",", e.first);
                    pa = 0;
                    // Faixas contíguas.
                    const std::vector<int>& vs = e.second;
                    for (size_t k = 0; k < vs.size();) {
                        size_t j = k;
                        while (j + 1 < vs.size() && vs[j + 1] == vs[j] + 1) j++;
                        printf("%s[%d,%d]", k ? "," : "", vs[k], vs[j]);
                        k = j + 1;
                    }
                    printf("]");
                }
                printf("},\"intervalos\":{");
                pa = 1;
                for (auto& e : intervalo) { printf("%s\"%u\":%d", pa ? "" : ",", e.first, e.second); pa = 0; }
                printf("}}\n");
            }
        }
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
