// Fase 7 do DLL de dados (specs/08_ELEMENTSKILL_DE_DADOS.md §13): o nosso elementskill.dll para o cliente 1.2.6.
// Lê o mesmo habilidades.pwsk do 1.5.5 (o leitor é o de elementskill_dados.cpp, incluído abaixo) e serve o contrato
// do 1.2.6, medido no binário original (DLL de 2007-07-04, VC6):
// - a classe concreta que o Create devolve (vtable em 10050144) tem 31 slots; a ordem é a do 1.5.5 sem os virtuais que
//   o 1.2.6 não tem, com GetRequiredSkill() e GetJunior() devolvendo um int (spec 08 §13);
// - as estruturas e as condições são porte da desmontagem do DLL 1.2.6 (endereços citados em cada função).
// Os números que nenhum slot do 1.2.6 expõe (classe, rank, nível máximo, nível do pré-requisito, proficiência por
// nível) ficam nos campos lógicos do .pwsk (ints[I_CLS], ints[I_RANK], max_level, pre[0].second, est[6]).
//
// Compilar com tools/elementskill/dll/compilar.bat (gera elementskill_126.dll; no cliente o nome é elementskill.dll).
#define PWSK_126
#include "elementskill_dados.cpp"

namespace GNET {

// Estruturas do 1.2.6 pelos deslocamentos lidos no DLL (Condition 100012F1, LearnCondition 1000101F, PetLearn 10001624).
struct UseRequirement { int mp, ap, form, weapon, freepackage, arrow; };
struct LearnRequirement { int level, sp, money, profession, rank; };
struct PetRequirement { int level, sp; int* list; int lsize; };

// 100048D3: alguma habilidade do mapa do jogador tem `id` como júnior (o campo +0x28 do stub com a marca +0x2C, o mesmo
// que o GetJunior devolve). Um nível só (o 1.5.5 desce a cadeia).
static bool sobrescrita126(unsigned int id) {
    for (std::map<unsigned int, Persistente>::iterator it = guardado.begin(); it != guardado.end(); ++it) {
        const HabD* s = achar(it->first);
        if (s && !s->niveis.empty() && !s->niveis[0].juniores.empty() && s->niveis[0].juniores[0].first == id) return true;
    }
    return false;
}

static const wchar_t VAZIO_W[1] = {0};  // 1007E424 no original
static const char VAZIO_A[1] = {0};     // 1007E420

class __declspec(dllexport) ElementSkill {
public:
    typedef unsigned int ID;

    // vtable da classe concreta do 1.2.6 (10050144) — a ordem é o contrato. Os bytes saem crus, como o original os lê.
    virtual const wchar_t* GetName() { return h->t0.c_str(); }                                   // 0  stub+0x0C
    virtual const char* GetNativeName() { return h->t1.c_str(); }                                // 1  stub+0x10
    virtual char GetType() const { return (char)n().bytes[B_TYPE]; }                             // 2  stub+0x1C
    virtual const char* GetIcon() { return h->t3.c_str(); }                                      // 3  stub+0x14
    virtual const wchar_t* GetIntroduction(wchar_t* buf, int len, const SkillStr& table);        // 4  100019CD
    virtual int GetCoolingTime() { return n().ints[I_COOL]; }                                    // 5
    virtual int GetExecuteTime() { return n().ints[I_EXEC]; }                                    // 6
    virtual int GetTargetType() { return n().ints[I_TARGET]; }                                   // 7  10001982
    virtual float GetPrayRange(float range);                                                     // 8  1000184F
    virtual int GetRequiredLevel() { return n().ints[I_RLEVEL]; }                                // 9
    virtual int GetRequiredSp() { return n().ints[I_RSP]; }                                      // 10
    virtual int GetRequiredBook() { return n().ints[I_RBOOK]; }                                  // 11
    virtual int GetRequiredMoney() { return n().ints[I_RMONEY]; }                                // 12
    virtual int GetRequiredSkill() { const NivelD& v = n(); return v.pre.empty() ? 0 : (int)v.pre[0].first; }  // 13 stub+0x28
    virtual int GetShowOrder() { return n().ints[I_SHOWORDER]; }                                 // 14 stub+0x31
    virtual int SetLevel(int l) { int t = nivel; nivel = l; return t; }                          // 15 10001834
    virtual char IsWarmup() { return (char)n().bytes[B_WARMUP]; }                                // 16
    virtual char IsAutoAttack() { return (char)n().bytes[B_AUTO]; }                              // 17 stub+0x51
    virtual char IsInstant() { return (char)n().bytes[B_INSTANT]; }                              // 18
    virtual char IsDurative() { return (char)n().bytes[B_DURATIVE]; }                            // 19
    virtual int GetRangeType() { return n().ints[I_RANGETYPE]; }                                 // 20 stub+0x5C
    virtual const char* GetEffect() { return h->t29.c_str(); }                                   // 21 stub+0x54
    virtual const char* GetElseEffect() { return h->t30.c_str(); }                               // 22 stub+0x58
    virtual int GetMpCost() { return n().ints[I_MP]; }                                           // 23
    virtual int GetApCost() { return n().ints[I_AP]; }                                           // 24 stub+0x36
    virtual int GetArrowCost() { return n().ints[I_ARROW]; }                                     // 25 stub+0x40
    virtual bool ValidWeapon(int w) const {                                                      // 26 10001A1D
        if (h->flags & 8) return true;
        for (size_t i = 0; i < h->armas.size(); i++) if (h->armas[i] == w) return true;
        return false;
    }
    virtual char GetAllowForms() { return (char)n().bytes[B_FORMS]; }                            // 27 stub+0x35
    virtual char Interrupt() { return (char)n().bytes[B_INTERRUPT]; }                            // 28
    virtual int GetJunior() { const NivelD& v = n(); return v.juniores.empty() ? 0 : (int)v.juniores[0].first; }  // 29
    virtual ~ElementSkill() {}                                                                   // 30 10001A5D

    // Estáticas que o executável 1.2.6 importa (23, spec 08 §13).
    static ElementSkill* Create(ID id, int n);
    void Destroy() { delete this; }
    static int Condition(ID id, UseRequirement& info, int ilevel);
    static int LearnCondition(ID id, LearnRequirement& info, int ilevel);
    static int PetLearn(ID id, PetRequirement& info, int level);
    static int GetAbility(ID id) { carregar(); return proficiencia(id); }
    static int GetAbilityPercent(ID id);
    static const char* GetEffect(ID id) { const HabD* h = achar(id); return h ? h->efeito.c_str() : VAZIO_A; }
    static int GetExecuteTime(ID id, int level) { return est(id, level, 5); }
    static const char* GetIcon(ID id) { const HabD* h = achar(id); return h ? h->icone.c_str() : VAZIO_A; }
    static const wchar_t* GetName(ID id) { const HabD* h = achar(id); return h ? h->nome.c_str() : VAZIO_W; }
    static const char* GetNativeName(ID id) { const HabD* h = achar(id); return h ? h->nome_nativo.c_str() : VAZIO_A; }
    static int GetRequiredBook(ID id, int level) { return est(id, level, 3); }
    static int GetRequiredLevel(ID id, int level) { return est(id, level, 0); }
    static int GetRequiredMoney(ID id, int level) { return est(id, level, 2); }
    static int GetRequiredSp(ID id, int level) { return est(id, level, 1); }
    static char GetType(ID id) { const HabD* h = achar(id); return h ? (char)h->tipo : 0; }
    static int GetVersion() { carregar(); return versao_do_dll; }
    static void LoadSkillData(void* data);
    static ID NextSkill(ID start);
    static int SetAbility(ID id, int ability) { carregar(); pegar(id).ability = ability; return ability; }
    static int SetLevel(ID id, int level) { carregar(); pegar(id).level = level; return level; }

    // Instância, no layout do original (Create 100017CC: 0x18 bytes, id em +0x10, nível em +0x14).
    const HabD* h;   // +0x04 (no original, o stub)
    float alcance;   // +0x08 (o GetPrayRange guarda aqui o alcance da arma)
    int livre;       // +0x0C
    ID id;           // +0x10
    int nivel;       // +0x14
    ElementSkill(const HabD* d, int l) : h(d), alcance(0), livre(0), id(d->id), nivel(l) {}
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
    // 100019CD: `stub->GetIntroduction(this, buf, len, table.Find(id*10+1))`; o stub chama `_snwprintf(buf, len, formato,
    // args…)` (no 1, nível 2: 2, 9, 74,7) e o slot devolve buf, ou L"" se o resultado for negativo.
    const wchar_t* formato = table.Find((int)h->id * 10 + 1);
    const NivelD& v = n();
    std::vector<unsigned int> w;
    for (size_t i = 0; i < v.intro.size(); i++) {
        const Arg& a = v.intro[i];
        if (a.tipo == 0) w.insert(w.end(), a.palavras.begin(), a.palavras.end());
        else if (a.tipo == 1) w.push_back((unsigned int)proficiencia(h->id));
        else w.push_back((unsigned int)v.est[6]);
    }
    for (int k = 0; k < 32; k++) w.push_back(0);
    FnVsnw f = vsnw();
    int r = f ? f(buf, (size_t)len, formato, (va_list)&w[0]) : -1;
    return r < 0 ? VAZIO_W : buf;
}

float ElementSkill::GetPrayRange(float range) {
    // 1000184F: guarda o alcance em +8 e chama `stub->GetPraydistance(this)` = a·alcance + b. Ataque/maldição (tipo 1
    // ou 3) com auto-ataque (+0x51): d − 1 se 0,3·d ≥ 1, senão 0,7·d; tipo 8: d; área (+0x5C) 2 ou 5: −1; senão d.
    // Constantes no DLL: 0,3f (100501D0), 1,0 (100501C8/C4), 0,7f (100501C0), −1,0f (100501D4).
    alcance = range;
    const NivelD& v = n();
    float d = f32(v.a * (double)range + v.b);
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
    // 100017CC: sem stub, NULL; nível acima do máximo vira o máximo (SetLevel no objeto novo).
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
    // 100012F1 (o membro Condition): flecha, arma, mana, forma (o bit `1 << form`, sem máscara: o shl do x86 usa os 5
    // bits baixos), fúria e o pacote livre do tipo de alvo 3.
    if (info.arrow < GetArrowCost()) return 9;
    if (!ValidWeapon(info.weapon)) return 1;
    if (info.mp < GetMpCost()) return 2;
    if (((1u << (info.form & 31)) & (unsigned int)(int)(signed char)GetAllowForms()) == 0) return 7;
    if (info.ap < GetApCost()) return 8;
    if (info.freepackage == 0 && GetTargetType() == 3) return 10;
    return 0;
}

int ElementSkill::Condition(ID id, UseRequirement& info, int ilevel) {
    // 100011A0.
    ElementSkill* s = Create(id, ilevel);
    if (!s) return 5;
    int r = s->Condicao(info);
    s->Destroy();
    return r;
}

int ElementSkill::LearnCondition(ID id, LearnRequirement& info, int ilevel) {
    // 1000101F.
    ElementSkill* s = Create(id, ilevel);
    if (!s) return 5;
    if (sobrescrita126(id)) { s->Destroy(); return 11; }
    int ret = 0;
    const NivelD& v1 = s->n();
    int srank = v1.ints[I_RANK], prank = info.rank;
    if (srank > prank) ret = 7;
    else {
        int a = (int)(srank * 0.1), b = (int)(prank * 0.1);  // fild/fmul 0.1 (100500A0)/_ftol
        if (a != b && a != 0) ret = 7;
    }
    int cls = v1.ints[I_CLS];
    if (ilevel < 1 || ilevel > s->h->max_level) ret = 3;
    else if (info.profession != cls && cls != 255) { s->Destroy(); return 4; }
    else if (info.level < s->GetRequiredLevel()) { s->Destroy(); return 2; }
    if (ret) { s->Destroy(); return ret; }
    if (info.sp < s->GetRequiredSp()) ret = 1;
    else if (info.money < s->GetRequiredMoney()) ret = 6;
    unsigned int rs = (unsigned int)s->GetRequiredSkill();
    if (rs > 0) {
        int nivel_exigido = s->n().pre.empty() ? 0 : s->n().pre[0].second;  // stub+0x44
        if (nivel_aprendido(rs) < nivel_exigido && !sobrescrita126(rs)) ret = 9;
    }
    if (ilevel > 1) {
        s->SetLevel(ilevel - 1);
        int x = s->n().est[6];  // a virtual 3 do stub no nível anterior
        if (x > 0 && proficiencia(id) < x) ret = 10;
    }
    s->Destroy();
    return ret;
}

int ElementSkill::PetLearn(ID id, PetRequirement& info, int level) {
    // 10001624.
    ElementSkill* s = Create(id, level);
    if (!s) return 5;
    int ret = 0;
    if (level < 1 || level > s->h->max_level) ret = 3;
    else if (info.level < s->GetRequiredLevel()) ret = 2;
    else if (info.sp < s->GetRequiredSp()) ret = 1;
    unsigned int rs = (unsigned int)s->GetRequiredSkill();
    if (rs > 0) {
        int cur = 0;
        for (int k = 0; k < info.lsize; k += 2)
            if (info.list[k] == (int)rs) { cur = info.list[k + 1]; break; }
        int nivel_exigido = s->n().pre.empty() ? 0 : s->n().pre[0].second;
        if (nivel_exigido > cur) ret = 9;
    }
    s->Destroy();
    return ret;
}

int ElementSkill::GetAbilityPercent(ID id) {
    // 10001598: proficiência entre a exigida no nível aprendido e a do anterior.
    int now = proficiencia(id), level = nivel_aprendido(id);
    const HabD* h = achar(id);
    if (!h) return 100;
    int max = 0, min = 0;
    if (level > 0) max = est(id, level, 6);
    if (level > 1) min = est(id, level - 1, 6);
    if (max == min) return 100;
    return (now - min) * 100 / (max - min);
}

void ElementSkill::LoadSkillData(void* data) {
    // 1000475B: limpa o mapa (1000452B) e lê `count` + entradas de 5 bytes (id u16, nível i8, proficiência i16).
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

}  // namespace GNET
