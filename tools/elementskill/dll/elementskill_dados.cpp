// B317 (fase 3 do DLL de dados): o nosso elementskill.dll. Mesmo contrato binário do original
// (specs/08_ELEMENTSKILL_DE_DADOS.md §2–§5), nenhum código por habilidade: tudo vem de `habilidades.pwsk`, ao lado do
// DLL (formato em crates/pw-data-loader/src/habilidades_do_cliente.rs). A lógica de base (condições, alcance,
// proficiência, sobrescrita) é porte do fonte `EvolvedPWClient/ElementSkill` (ElementSkill.cpp, skill.h, skillwrapper.cpp),
// conferida contra o gabarito do original (tools/elementskill/gabarito.cpp).
//
// Compilar com tools/elementskill/dll/compilar.bat (x86, /Zc:wchar_t- para `wchar_t` = `unsigned short`, como no VC6, e
// os nomes exportados saírem iguais aos que o executável importa).
#include <windows.h>
#include <stdio.h>
#include <stdarg.h>
#include <string.h>
#include <map>
#include <string>
#include <vector>

namespace _STL {
template <class A, class B> struct pair {
    A first;
    B second;
};
}

namespace GNET {

typedef _STL::pair<unsigned int, int> Par;

// Implementada pelo executável (spec 08 §4): [+4] assign, [+8] clear, [+0x10] push_back.
template <class T> class skill_array_interface {
public:
    virtual void nao_usado0() = 0;
    virtual void assign(const T* inicio, const T* fim) = 0;
    virtual void clear() = 0;
    virtual void nao_usado3() = 0;
    virtual void push_back(const T& x) = 0;
};

// Implementada pelo executável; o DLL só chama Find (o texto N×10+1 do skillstr.txt).
class SkillStr {
public:
    virtual wchar_t* Find(int n) const = 0;
};

struct ComboSkillState { unsigned int skillid; int arg[3]; };
struct LearnRequirement { int level, sp, money, profession, rank, realm_level; };
struct UseRequirement { int mp, ap, form, weapon, freepackage, arrow, move_env; bool is_combat; int hp, max_hp; ComboSkillState combo_state; };
struct PetRequirement { int level, sp; int* list; int lsize; };
struct GoblinRequirement { int genius[5]; int profession, sp, money, level, mp; };
struct GoblinUseRequirement { int mp, ap; int genius[5]; int profession, level, move_env; };

// ---------------------------------------------------------------- os dados (habilidades.pwsk)
struct Arg { unsigned char tipo; std::vector<unsigned int> palavras; };
struct NivelD {
    int ints[25];
    signed char bytes[13];
    double a, b, c;
    bool tem_genio;
    int genio[5];
    std::vector<Par> pre, juniores;
    int est[7];  // GetRequiredLevel, Sp, Money, Book, RealmLevel, GetExecuteTime, GetMaxAbility
    std::vector<Arg> intro;
};
struct HabD {
    unsigned int id;
    int max_level;
    signed char tipo;
    unsigned char flags;  // 1 duende, 2 móvel, 4 sobrescrita, 8 todas as armas, 16 vida, 32 combo
    int pre_combo, recarga_comum, intervalo_combo;
    std::string icone, nome_nativo, efeito;
    std::wstring nome;
    std::wstring t0;
    std::string t1, t3, t29, t30;
    std::vector<int> armas;
    unsigned char vida_lo, vida_hi;
    unsigned char combo_arg;
    std::vector<std::pair<int, int> > combo_faixas;  // B317: faixas fechadas; vazia = sempre falsa
    std::vector<NivelD> niveis;
};

// Índices em NivelD::ints / bytes (a ordem de SLOTS_INT / SLOTS_BYTE do arquivo).
enum { I_CLS, I_COOL, I_EXEC, I_TARGET, I_RANK, I_RLEVEL, I_RSP, I_RBOOK, I_RMONEY, I_RREALM, I_RSKILLCOUNT, I_SHOWORDER, I_MAXLEVEL,
       I_RANGETYPE, I_CASTENV, I_MP, I_AP, I_ARROW, I_JUNIORCOUNT, I_CLSLIMIT, I_CCD, I_CCDTIME, I_ITEMCOST, I_ITEMCOSTCOUNT, I_COMBOPRE };
enum { B_TYPE, B_WARMUP, B_AUTO, B_INSTANT, B_DURATIVE, B_LAND, B_WATER, B_AIR, B_NOTCOMBAT, B_MOVING, B_FORMS, B_INTERRUPT, B_COMBOCOND };

static std::vector<HabD> habilidades;
static std::map<unsigned int, size_t> por_id;
static std::map<unsigned int, std::vector<unsigned int> > pos_combo;  // pré-combo → habilidades (ordem do NextSkill)
static int versao_do_dll;
static bool carregado;

class VisibleState;
class TeamState;
static std::map<std::pair<int, int>, VisibleState*> visiveis;
static std::map<int, TeamState*> equipe;
static std::map<int, std::vector<unsigned int> > inerentes;

struct Leitor {
    const unsigned char* p;
    const unsigned char* fim;
    bool ok;
    template <class T> T ler() {
        T v{};
        if (p + sizeof(T) > fim) { ok = false; return v; }
        memcpy(&v, p, sizeof(T));
        p += sizeof(T);
        return v;
    }
    std::string str() {
        unsigned short n = ler<unsigned short>();
        if (p + n > fim) { ok = false; return std::string(); }
        std::string s((const char*)p, n);
        p += n;
        return s;
    }
    std::wstring wstr() {
        unsigned short n = ler<unsigned short>();
        if (p + 2 * n > fim) { ok = false; return std::wstring(); }
        std::wstring s((const wchar_t*)p, n);
        p += 2 * n;
        return s;
    }
};

class VisibleState {
public:
    int id;
    std::wstring nome;
    std::string hh, efeito;
    virtual const int GetID() const { return id; }
    virtual const wchar_t* GetName() const { return nome.c_str(); }
    virtual const char* GetHH() const { return hh.c_str(); }
    virtual const char* GetEffect() const { return efeito.c_str(); }
    static const VisibleState* Query(int profession, int id);
};
class TeamState {
public:
    int id;
    std::wstring nome;
    std::string icone;
    virtual const int GetID() const { return id; }
    virtual const wchar_t* GetName() const { return nome.c_str(); }
    virtual const char* GetIcon() const { return icone.c_str(); }
    static const TeamState* Query(int id);
};

static HMODULE este_dll;

static void carregar() {
    if (carregado) return;
    carregado = true;
    wchar_t caminho[MAX_PATH];
    GetModuleFileNameW(este_dll, caminho, MAX_PATH);
    wchar_t* barra = wcsrchr(caminho, L'\\');
    if (barra) wcscpy(barra + 1, L"habilidades.pwsk");
    HANDLE f = CreateFileW(caminho, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
    if (f == INVALID_HANDLE_VALUE) return;
    DWORD tam = GetFileSize(f, NULL), lidos = 0;
    std::vector<unsigned char> b(tam);
    ReadFile(f, b.data(), tam, &lidos, NULL);
    CloseHandle(f);
    Leitor r = {b.data(), b.data() + lidos, true};
    if (lidos < 8 || memcmp(b.data(), "PWSK", 4) != 0) return;
    r.p += 4;
    if (r.ler<unsigned int>() != 1) return;
    versao_do_dll = r.ler<int>();
    unsigned int nh = r.ler<unsigned int>(), nv = r.ler<unsigned int>(), ne = r.ler<unsigned int>(), ni = r.ler<unsigned int>();
    habilidades.resize(nh);
    for (unsigned int k = 0; k < nh && r.ok; k++) {
        HabD& h = habilidades[k];
        h.id = r.ler<unsigned int>();
        h.max_level = r.ler<unsigned char>();
        h.tipo = r.ler<signed char>();
        h.flags = r.ler<unsigned char>();
        h.pre_combo = r.ler<int>();
        h.recarga_comum = r.ler<int>();
        h.intervalo_combo = r.ler<int>();
        h.icone = r.str();
        h.nome = r.wstr();
        h.nome_nativo = r.str();
        h.efeito = r.str();
        h.t0 = r.wstr();
        h.t1 = r.str();
        h.t3 = r.str();
        h.t29 = r.str();
        h.t30 = r.str();
        if (!(h.flags & 8)) {
            unsigned short n = r.ler<unsigned short>();
            for (unsigned short j = 0; j < n; j++) h.armas.push_back(r.ler<int>());
        }
        if (h.flags & 16) { h.vida_lo = r.ler<unsigned char>(); h.vida_hi = r.ler<unsigned char>(); }
        if (h.flags & 32) {
            h.combo_arg = r.ler<unsigned char>();
            unsigned char q = r.ler<unsigned char>();
            for (unsigned char j = 0; j < q; j++) { int lo = r.ler<int>(); int hi = r.ler<int>(); h.combo_faixas.push_back(std::make_pair(lo, hi)); }
        }
        h.niveis.resize(h.max_level);
        for (int n = 0; n < h.max_level && r.ok; n++) {
            NivelD& v = h.niveis[n];
            for (int j = 0; j < 25; j++) v.ints[j] = r.ler<int>();
            for (int j = 0; j < 13; j++) v.bytes[j] = r.ler<signed char>();
            v.a = r.ler<double>();
            v.b = r.ler<double>();
            v.c = r.ler<double>();
            v.tem_genio = r.ler<unsigned char>() == 1;
            if (v.tem_genio) for (int j = 0; j < 5; j++) v.genio[j] = r.ler<int>();
            for (int l = 0; l < 2; l++) {
                unsigned char q = r.ler<unsigned char>();
                std::vector<Par>& dst = l ? v.juniores : v.pre;
                for (unsigned char j = 0; j < q; j++) { Par p; p.first = r.ler<unsigned int>(); p.second = r.ler<int>(); dst.push_back(p); }
            }
            for (int j = 0; j < 7; j++) v.est[j] = r.ler<int>();
            unsigned char na = r.ler<unsigned char>();
            for (unsigned char j = 0; j < na; j++) {
                Arg a;
                a.tipo = r.ler<unsigned char>();
                if (a.tipo == 0) {
                    unsigned char q = r.ler<unsigned char>();
                    for (unsigned char t = 0; t < q; t++) a.palavras.push_back(r.ler<unsigned int>());
                }
                v.intro.push_back(a);
            }
        }
        por_id[h.id] = k;
    }
    for (unsigned int k = 0; k < nv && r.ok; k++) {
        VisibleState* e = new VisibleState();
        int prof = r.ler<int>(), consulta = r.ler<int>();
        e->id = r.ler<int>();
        e->nome = r.wstr();
        e->hh = r.str();
        e->efeito = r.str();
        visiveis[std::make_pair(prof, consulta)] = e;
    }
    for (unsigned int k = 0; k < ne && r.ok; k++) {
        TeamState* e = new TeamState();
        int consulta = r.ler<int>();
        e->id = r.ler<int>();
        e->nome = r.wstr();
        e->icone = r.str();
        equipe[consulta] = e;
    }
    for (unsigned int k = 0; k < ni && r.ok; k++) {
        int cls = r.ler<int>();
        unsigned short n = r.ler<unsigned short>();
        for (unsigned short j = 0; j < n; j++) inerentes[cls].push_back(r.ler<unsigned int>());
    }
    // Pós-combo na ordem do NextSkill (`SkillStub::GetPostComboSkill`).
    for (size_t k = 0; k < habilidades.size(); k++)
        if (habilidades[k].pre_combo) pos_combo[(unsigned int)habilidades[k].pre_combo].push_back(habilidades[k].id);
    if (!r.ok) { habilidades.clear(); por_id.clear(); }
}

static const HabD* achar(unsigned int id) {
    carregar();
    std::map<unsigned int, size_t>::const_iterator it = por_id.find(id);
    return it == por_id.end() ? NULL : &habilidades[it->second];
}

// ---------------------------------------------------------------- o estado do jogador (SkillWrapper)
struct Persistente { int ability, level; };
static std::map<unsigned int, Persistente> guardado;  // `PersistentData(int _t = 0, int _l = 1)`

static Persistente& pegar(unsigned int id) {
    std::map<unsigned int, Persistente>::iterator it = guardado.find(id);
    if (it == guardado.end()) { Persistente p = {0, 1}; it = guardado.insert(std::make_pair(id, p)).first; }
    return it->second;
}
static int nivel_aprendido(unsigned int id) { std::map<unsigned int, Persistente>::iterator it = guardado.find(id); return it == guardado.end() ? 0 : it->second.level; }
static int proficiencia(unsigned int id) { std::map<unsigned int, Persistente>::iterator it = guardado.find(id); return it == guardado.end() ? 0 : it->second.ability; }

// `SkillWrapper::IsOverridden(id, senior)` (skillwrapper.cpp): a sênior é quem tem juniores.
static bool sobrescrita_por(unsigned int id, unsigned int senior, int profundidade) {
    const HabD* s = achar(senior);
    if (!s || profundidade > 64 || s->niveis.empty() || s->niveis[0].juniores.empty()) return false;
    const std::vector<Par>& pre = s->niveis[0].juniores;
    for (size_t i = 0; i < pre.size(); i++) {
        unsigned int pid = pre[i].first;
        if (pid <= 0) continue;
        if (id == pid) return true;
        if (sobrescrita_por(id, pid, profundidade + 1)) return true;
    }
    return false;
}
static bool sobrescrita(unsigned int id) {
    for (std::map<unsigned int, Persistente>::iterator it = guardado.begin(); it != guardado.end(); ++it)
        if (sobrescrita_por(id, it->first, 0)) return true;
    return false;
}

// O `_snwprintf` do mesmo msvcrt.dll que o original usava (a formatação de `%.1f` muda entre CRTs).
typedef int(__cdecl* FnVsnw)(wchar_t*, size_t, const wchar_t*, va_list);
static FnVsnw vsnw() {
    static FnVsnw f = NULL;
    if (!f) {
        HMODULE m = LoadLibraryA("msvcrt.dll");
        if (m) f = (FnVsnw)GetProcAddress(m, "_vsnwprintf");
    }
    return f;
}

static inline float f32(double x) { return (float)x; }

// ---------------------------------------------------------------- a classe que o executável usa
class __declspec(dllexport) ElementSkill {
public:
    typedef unsigned int ID;

    // vtable (spec 08 §3) — a ordem é o contrato.
    virtual const wchar_t* GetName() { return h->t0.c_str(); }                                      // 0
    virtual const char* GetNativeName() { return h->t1.c_str(); }                                   // 1
    virtual char GetType() const { return (char)n().bytes[B_TYPE]; }                                // 2
    virtual const char* GetIcon() { return h->t3.c_str(); }                                         // 3
    virtual const wchar_t* GetIntroduction(wchar_t* buf, int len, const SkillStr& table);           // 4
    virtual int GetCls() const { return n().ints[I_CLS]; }                                          // 5
    virtual int GetCoolingTime() { return n().ints[I_COOL]; }                                       // 6
    virtual int GetExecuteTime() { return n().ints[I_EXEC]; }                                       // 7
    virtual int GetTargetType() { return n().ints[I_TARGET]; }                                      // 8
    virtual float GetPrayRange(float range, float prayplus);                                        // 9
    virtual int GetRank() { return n().ints[I_RANK]; }                                              // 10
    virtual int GetRequiredLevel() { return n().ints[I_RLEVEL]; }                                   // 11
    virtual int GetRequiredSp() { return n().ints[I_RSP]; }                                         // 12
    virtual int GetRequiredBook() { return n().ints[I_RBOOK]; }                                     // 13
    virtual int GetRequiredMoney() { return n().ints[I_RMONEY]; }                                   // 14
    virtual int GetRequiredRealmLevel() { return n().ints[I_RREALM]; }                              // 15
    virtual int GetRequiredSkillCount() { return (int)n().pre.size(); }                            // 16
    virtual const Par* GetRequiredSkillAt(int i) { return &n().pre[i]; }                            // 17
    virtual void GetRequiredSkill(skill_array_interface<Par>& l) {                                  // 18
        const std::vector<Par>& v = n().pre;
        if (v.empty()) l.clear(); else l.assign(&v[0], &v[0] + v.size());
    }
    virtual int GetShowOrder() { return n().ints[I_SHOWORDER]; }                                    // 19
    virtual int SetLevel(int l) { int t = nivel; nivel = l; return t; }                             // 20
    virtual int GetMaxLevel() { return n().ints[I_MAXLEVEL]; }                                      // 21
    virtual bool IsWarmup() { return n().bytes[B_WARMUP] != 0; }                                    // 22
    virtual bool IsAutoAttack() { return n().bytes[B_AUTO] != 0; }                                  // 23
    virtual bool IsInstant() { return n().bytes[B_INSTANT] != 0; }                                  // 24
    virtual bool IsDurative() { return n().bytes[B_DURATIVE] != 0; }                                // 25
    virtual int GetRangeType() { return n().ints[I_RANGETYPE]; }                                    // 26
    virtual int GetCastEnv() { return n().ints[I_CASTENV]; }                                        // 27
    virtual int* GetRequiredGenius(int) {                                                           // 28
        const NivelD& v = n();
        return v.tem_genio ? const_cast<int*>(v.genio) : NULL;
    }
    virtual const char* GetEffect() { return h->t29.c_str(); }                                      // 29
    virtual const char* GetElseEffect() { return h->t30.c_str(); }                                  // 30
    virtual int GetMpCost() { return n().ints[I_MP]; }                                              // 31
    virtual int GetApCost() { return n().ints[I_AP]; }                                              // 32
    virtual int GetArrowCost() { return n().ints[I_ARROW]; }                                        // 33
    virtual bool ValidWeapon(int w) const {                                                         // 34
        if (h->flags & 8) return true;
        for (size_t i = 0; i < h->armas.size(); i++) if (h->armas[i] == w) return true;
        return false;
    }
    virtual bool IsAllowLand() { return n().bytes[B_LAND] != 0; }                                   // 35
    virtual bool IsAllowWater() { return n().bytes[B_WATER] != 0; }                                 // 36
    virtual bool IsAllowAir() { return n().bytes[B_AIR] != 0; }                                     // 37
    virtual bool GetNotuseInCombat() { return n().bytes[B_NOTCOMBAT] != 0; }                        // 38
    virtual bool IsMovingSkill() { return n().bytes[B_MOVING] != 0; }                               // 39
    virtual char GetAllowForms() { return (char)n().bytes[B_FORMS]; }                               // 40
    virtual bool Interrupt() { return n().bytes[B_INTERRUPT] != 0; }                                // 41
    virtual int GetJuniorCount() const { return (int)n().juniores.size(); }                        // 42
    virtual const Par* GetJuniorAt(int i) const { return &n().juniores[i]; }                        // 43
    virtual void GetJunior(skill_array_interface<Par>& l) const {                                   // 44
        const std::vector<Par>& v = n().juniores;
        if (v.empty()) l.clear(); else l.assign(&v[0], &v[0] + v.size());
    }
    virtual int GetClsLimit() { return n().ints[I_CLSLIMIT]; }                                      // 45
    virtual int GetCommonCoolDown() { return n().ints[I_CCD]; }                                     // 46
    virtual int GetCommonCoolDownTime() { return n().ints[I_CCDTIME]; }                             // 47
    virtual int GetItemCost() { return n().ints[I_ITEMCOST]; }                                      // 48
    virtual int GetItemCostCount() { return n().ints[I_ITEMCOSTCOUNT]; }                            // 49
    virtual int GetComboSkPreSkill() { return n().ints[I_COMBOPRE]; }                               // 50
    virtual bool CheckHpCondition(int hp, int max_hp) {                                             // 51
        if (!(h->flags & 16)) return true;
        return hp >= max_hp / 100.f * (h->vida_lo) && hp <= max_hp / 100.f * (h->vida_hi);
    }
    virtual bool CheckComboSkExtraCondition() {                                                     // 52
        if (!(h->flags & 32)) return true;
        int v = combo[h->combo_arg < 3 ? h->combo_arg : 0];
        for (size_t i = 0; i < h->combo_faixas.size(); i++)
            if (v >= h->combo_faixas[i].first && v <= h->combo_faixas[i].second) return true;
        return false;
    }

    // Estáticas (spec 08 §2).
    static ElementSkill* Create(ID id, int n);
    void Destroy() { delete this; }
    static int Condition(ID id, UseRequirement& info, int ilevel);
    static int LearnCondition(ID id, LearnRequirement& info, int ilevel);
    static int PetLearn(ID id, PetRequirement& info, int level);
    static int GoblinLearn(ID id, GoblinRequirement& info, int level);
    static int GoblinCondition(ID id, GoblinUseRequirement& info, int level);
    static int GetAbility(ID id) { carregar(); return proficiencia(id); }
    static int GetAbilityPercent(ID id);
    static void GetComboSkActivated(const ComboSkillState& st, skill_array_interface<Par>& l);
    static int GetComboSkPreSkill(ID id) { const HabD* h = achar(id); return h ? h->pre_combo : 0; }
    static int GetCommonCoolDown(ID id) { const HabD* h = achar(id); return h ? h->recarga_comum : 0; }
    static const char* GetEffect(ID id) { const HabD* h = achar(id); return h ? h->efeito.c_str() : NULL; }
    static int GetExecuteTime(ID id, int level) { return est(id, level, 5); }
    static const char* GetIcon(ID id) { const HabD* h = achar(id); return h ? h->icone.c_str() : NULL; }
    static void GetInherentSkills(int cls, skill_array_interface<unsigned int>& l);
    static int GetMaxAbility(ID id, int level) { if (id <= 0) return 0; return est(id, level, 6); }
    static const wchar_t* GetName(ID id) { const HabD* h = achar(id); return h ? h->nome.c_str() : NULL; }
    static const char* GetNativeName(ID id) { const HabD* h = achar(id); return h ? h->nome_nativo.c_str() : NULL; }
    static int GetRequiredBook(ID id, int level) { return est(id, level, 3); }
    static int GetRequiredLevel(ID id, int level) { return est(id, level, 0); }
    static int GetRequiredMoney(ID id, int level) { return est(id, level, 2); }
    static int GetRequiredRealmLevel(ID id, int level) { return est(id, level, 4); }
    static int GetRequiredSp(ID id, int level) { return est(id, level, 1); }
    static char GetType(ID id) { const HabD* h = achar(id); return h ? (char)h->tipo : 0; }
    static int GetVersion() { carregar(); return versao_do_dll; }
    static void InitStaticData() { carregar(); }
    static bool IsGoblinSkill(ID id) { const HabD* h = achar(id); return h && (h->flags & 1); }
    static bool IsMovingSkill(ID id) { const HabD* h = achar(id); return h && (h->flags & 2); }
    static bool IsOverridden(ID id) { carregar(); return sobrescrita(id); }
    static void LoadSkillData(void* data);
    static ID NextSkill(ID start);
    static int SetAbility(ID id, int ability) { carregar(); pegar(id).ability = ability; return ability; }
    static int SetLevel(ID id, int level) { carregar(); pegar(id).level = level; return level; }

    // Instância.
    const HabD* h;
    int nivel;
    int combo[3];
    ElementSkill(const HabD* d, int l) : h(d), nivel(l) { combo[0] = combo[1] = combo[2] = 0; }
    virtual ~ElementSkill() {}
    // O nível nos dados: o original lê fora da tabela abaixo de 1; aqui vale o nível 1 (spec 08 §10).
    const NivelD& n() const {
        int k = nivel < 1 ? 1 : (nivel > h->max_level ? h->max_level : nivel);
        return h->niveis[k - 1];
    }
    int Condicao(UseRequirement& info);
    static int est(ID id, int level, int k) {
        const HabD* h = achar(id);
        if (!h) return 0;
        int l = level > h->max_level ? h->max_level : (level < 1 ? 1 : level);
        return h->niveis[l - 1].est[k];
    }
};

const wchar_t* ElementSkill::GetIntroduction(wchar_t* buf, int len, const SkillStr& table) {
    // `stub->GetIntroduction(this, buf, len, table.Find(id*10+1))` = `_snwprintf(buf, len, formato, args…)`.
    const wchar_t* formato = table.Find((int)h->id * 10 + 1);
    const NivelD& v = n();
    std::vector<unsigned int> w;
    for (size_t i = 0; i < v.intro.size(); i++) {
        const Arg& a = v.intro[i];
        if (a.tipo == 0) w.insert(w.end(), a.palavras.begin(), a.palavras.end());
        else if (a.tipo == 1) w.push_back((unsigned int)proficiencia(h->id));
        else w.push_back((unsigned int)v.est[6]);
    }
    for (int k = 0; k < 32; k++) w.push_back(0);  // folga: formato com mais conversões que argumentos
    FnVsnw f = vsnw();
    int r = f ? f(buf, (size_t)len, formato, (va_list)&w[0]) : -1;
    return r < 0 ? L"" : buf;
}

float ElementSkill::GetPrayRange(float range, float prayplus) {
    // `Skill::GetPrayRange` (skill.h:251-272) com `GetPraydistance` = (float)(a·alcance + b + c·bônus).
    const NivelD& v = n();
    float d = f32(v.a * (double)range + v.b + v.c * (double)prayplus);
    char tipo = (char)v.bytes[B_TYPE];
    if (tipo == 1 || tipo == 3) {
        if (v.bytes[B_AUTO]) {
            float r = 0.3f * d;
            if (r >= 1.0) return d - 1.0f;
            return (float)0.7 * d;
        }
        return d;
    }
    if (tipo == 8) return d;
    int area = v.ints[I_RANGETYPE];
    if (area == 2 || area == 5) return -1;
    return d;
}

ElementSkill* ElementSkill::Create(ID id, int n) {
    const HabD* h = achar(id);
    if (!h) return NULL;
    return new ElementSkill(h, n > h->max_level ? h->max_level : n);
}

ElementSkill::ID ElementSkill::NextSkill(ID start) {
    carregar();
    if (habilidades.empty()) return 0;
    if (start == 0) return habilidades[0].id;
    std::map<unsigned int, size_t>::const_iterator it = por_id.find(start);
    if (it == por_id.end() || it->second + 1 >= habilidades.size()) return 0;
    return habilidades[it->second + 1].id;
}

int ElementSkill::Condicao(UseRequirement& info) {
    // `ElementSkill::Condition(UseRequirement&)` (ElementSkill.cpp:191-227).
    if (info.arrow < GetArrowCost()) return 9;
    if (!ValidWeapon(info.weapon)) return 1;
    if (info.mp < GetMpCost()) return 2;
    int form_type = (info.form & 0xC0) >> 6;  // FORM_MASK_HIGH
    if ((GetAllowForms() & (1 << form_type)) == 0) return 7;
    if (info.ap < GetApCost()) return 8;
    if (info.freepackage == 0 && GetTargetType() == 3) return 10;
    if ((info.move_env == 0 && !IsAllowLand()) || (info.move_env == 1 && !IsAllowWater()) || (info.move_env == 2 && !IsAllowAir())) return 3;
    if (info.is_combat && GetNotuseInCombat()) return 11;
    if (!CheckHpCondition(info.hp, info.max_hp)) return 12;
    int preskill = GetComboSkPreSkill();
    if (preskill) {
        if (info.combo_state.skillid != (ID)preskill || !CheckComboSkExtraCondition()) return 13;
    }
    return 0;
}

int ElementSkill::Condition(ID id, UseRequirement& info, int ilevel) {
    ElementSkill* s = Create(id, ilevel);
    if (!s) return 5;
    for (int i = 0; i < 3; i++) s->combo[i] = info.combo_state.arg[i];
    int r = s->Condicao(info);
    s->Destroy();
    return r;
}

int ElementSkill::LearnCondition(ID id, LearnRequirement& info, int ilevel) {
    // ElementSkill.cpp:43-108.
    ElementSkill* s = Create(id, ilevel);
    if (!s) return 5;
    int ret = 0;
    if (sobrescrita(id)) return 11;
    int srank = s->GetRank(), prank = info.rank;
    if (srank > prank) ret = 7;
    else {
        srank = (int)(srank * 0.1);
        prank = (int)(prank * 0.1);
        if (srank != prank && srank != 0) ret = 7;
    }
    if (ilevel < 1 || ilevel > s->GetMaxLevel()) ret = 3;
    else if (info.profession != s->GetCls() && s->GetCls() != 255) ret = 4;
    else if (info.level < s->GetRequiredLevel()) ret = 2;
    if (ret) { s->Destroy(); return ret; }
    if (info.sp < s->GetRequiredSp()) ret = 1;
    else if (info.money < s->GetRequiredMoney()) ret = 6;
    const std::vector<Par>& pre = s->n().pre;
    for (size_t i = 0; i < pre.size(); i++) {
        unsigned int pid = pre[i].first;
        int plevel = pre[i].second;
        if (pid > 0 && nivel_aprendido(pid) < plevel && !sobrescrita(pid)) { ret = 9; break; }
    }
    if (ilevel > 1) {
        s->SetLevel(ilevel - 1);
        int ability = s->n().est[6];
        if (ability > 0 && proficiencia(id) < ability) ret = 10;
    }
    if (info.realm_level < s->GetRequiredRealmLevel()) ret = 12;
    s->Destroy();
    return ret;
}

int ElementSkill::PetLearn(ID id, PetRequirement& info, int level) {
    // ElementSkill.cpp:455-499.
    ElementSkill* s = Create(id, level);
    if (!s) return 5;
    int ret = 0;
    if (level < 1 || level > s->GetMaxLevel()) ret = 3;
    else if (info.level < s->GetRequiredLevel()) ret = 2;
    else if (info.sp < s->GetRequiredSp()) ret = 1;
    const std::vector<Par>& pre = s->n().pre;
    for (size_t i = 0; i < pre.size(); i++) {
        unsigned int pid = pre[i].first;
        int plevel = pre[i].second;
        if (pid > 0) {
            int cur = 0;
            for (int k = 0; k < info.lsize; k += 2)
                if (info.list[k] == (int)pid) { cur = info.list[k + 1]; break; }
            if (plevel > cur) { ret = 9; break; }
        }
    }
    s->Destroy();
    return ret;
}

int ElementSkill::GoblinLearn(ID id, GoblinRequirement& info, int level) {
    // ElementSkill.cpp:501-552.
    ElementSkill* s = Create(id, level);
    if (!s) return 5;
    if (level < 1 || level > s->GetMaxLevel()) return 3;
    if (s->GetCls() != 258) return 7;
    int ret = 0, req[5];
    int rl = s->GetRequiredLevel();
    if (info.level < rl % 100) return 9;
    rl /= 100;
    for (int i = 0; i < 5; i++) { req[4 - i] = rl % 10; rl /= 10; }
    for (int i = 0; i < 5; i++) if (info.genius[i] < req[i]) return 2;
    if (info.sp < s->GetRequiredSp()) ret = 1;
    bool limite = s->GetClsLimit() != 0 && ((1 << info.profession) & s->GetClsLimit()) == 0;
    if (info.mp < s->GetMpCost() && limite) ret = 12;
    else if (info.mp < s->GetMpCost()) ret = 10;
    else if (limite) ret = 11;
    s->Destroy();
    return ret;
}

int ElementSkill::GoblinCondition(ID id, GoblinUseRequirement& info, int level) {
    // ElementSkill.cpp:554-606.
    ElementSkill* s = Create(id, level);
    if (!s) return 5;
    if (s->GetCls() != 258) return 7;
    int req[5];
    int rl = s->GetRequiredLevel() / 100;
    for (int i = 0; i < 5; i++) { req[4 - i] = rl % 10; rl /= 10; }
    for (int i = 0; i < 5; i++) if (req[i] != 0 && info.genius[i] < req[i]) return 4;
    if ((info.move_env == 0 && !s->IsAllowLand()) || (info.move_env == 1 && !s->IsAllowWater()) || (info.move_env == 2 && !s->IsAllowAir())) return 8;
    int ap = (s->GetApCost() / 1000) + (s->GetApCost() % 1000) * (level - 1);
    int ret = 0;
    if (s->GetClsLimit() != 0 && ((1 << info.profession) & s->GetClsLimit()) == 0) ret = 1;
    else if (info.mp < s->GetMpCost()) ret = 2;
    else if (info.ap < ap) ret = 3;
    s->Destroy();
    return ret;
}

int ElementSkill::GetAbilityPercent(ID id) {
    // ElementSkill.cpp:434-453.
    int now = proficiencia(id), level = nivel_aprendido(id);
    const HabD* h = achar(id);
    if (!h) return 100;
    int max = 0, min = 0;
    if (level > 0) max = est(id, level, 6);
    if (level > 1) min = est(id, level - 1, 6);
    if (max == min) return 100;
    return 100 * (now - min) / (max - min);
}

void ElementSkill::GetComboSkActivated(const ComboSkillState& st, skill_array_interface<Par>& l) {
    // ElementSkill.cpp:613-631; acrescenta sem limpar (spec 08 §4).
    if (st.skillid == 0) return;
    carregar();
    std::map<unsigned int, std::vector<unsigned int> >::const_iterator it = pos_combo.find(st.skillid);
    if (it == pos_combo.end()) return;
    for (size_t i = 0; i < it->second.size(); i++) {
        ElementSkill* s = Create(it->second[i], 1);
        if (!s) continue;
        for (int k = 0; k < 3; k++) s->combo[k] = st.arg[k];
        if (s->CheckComboSkExtraCondition()) { Par p; p.first = s->h->id; p.second = s->h->intervalo_combo; l.push_back(p); }
        s->Destroy();
    }
}

void ElementSkill::GetInherentSkills(int cls, skill_array_interface<unsigned int>& l) {
    carregar();
    std::map<int, std::vector<unsigned int> >::const_iterator it = inerentes.find(cls);
    if (it == inerentes.end() || it->second.empty()) l.clear();
    else l.assign(&it->second[0], &it->second[0] + it->second.size());
}

void ElementSkill::LoadSkillData(void* data) {
    // `SkillWrapper::LoadData` (skillwrapper.cpp:17): `cmd_skill_data` empacotado em 1 byte.
    carregar();
    guardado.clear();
    const unsigned char* p = (const unsigned char*)data;
    unsigned int count;
    memcpy(&count, p, 4);
    p += 4;
    for (unsigned int i = 0; i < count; i++, p += 5) {
        short id, ability;
        signed char level;
        memcpy(&id, p, 2);
        level = (signed char)p[2];
        memcpy(&ability, p + 3, 2);
        Persistente e = {ability, level};
        guardado[(unsigned int)id] = e;
    }
}

const VisibleState* VisibleState::Query(int profession, int id) {
    carregar();
    std::map<std::pair<int, int>, VisibleState*>::const_iterator it = visiveis.find(std::make_pair(profession, id));
    return it == visiveis.end() ? NULL : it->second;
}

const TeamState* TeamState::Query(int id) {
    carregar();
    std::map<int, TeamState*>::const_iterator it = equipe.find(id);
    return it == equipe.end() ? NULL : it->second;
}

}  // namespace GNET

// Exportações com o nome exato que o executável importa (spec 08 §2).
#pragma comment(linker, "/EXPORT:?Query@VisibleState@GNET@@SAPBV12@HH@Z=?Query@VisibleState@GNET@@SAPBV12@HH@Z")
#pragma comment(linker, "/EXPORT:?Query@TeamState@GNET@@SAPBV12@H@Z=?Query@TeamState@GNET@@SAPBV12@H@Z")

BOOL APIENTRY DllMain(HMODULE m, DWORD motivo, LPVOID) {
    if (motivo == DLL_PROCESS_ATTACH) GNET::este_dll = m;
    return TRUE;
}
