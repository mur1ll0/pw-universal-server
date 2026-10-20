// B305: onde, no arquivo elementskill.dll, mora cada número por nível de cada habilidade — para editar o número do
// cliente trocando só os bytes da tabela (recompilar do fonte desfaria o que o BR mudou; ver B303).
//
// Uso: enderecos.exe <elementskill.dll> <entrada.txt>  → uma linha JSON por (habilidade, campo) na saída.
// entrada.txt, uma linha por (habilidade, campo):  "<id> <campo> <i|f> <n> v1 … vn"
//   (gerada por `specs/habilidades_155/mapear_elementskill.py` a partir da medição do `numeros.exe`).
//
// Método, por linha: a sequência v1…vn (int32 ou float32) é procurada nas seções de dados da imagem carregada; cada
// candidato é provado escrevendo uma sentinela no elemento do nível 1 e chamando a função do próprio DLL — a estática
// exportada `GetRequired*`/`GetExecuteTime(id, 1)`, ou o slot da vtable (`GetMpCost`, `GetCoolingTime`) do objeto de
// `Create(id, 1)`. Se o valor devolvido muda, a tabela é aquela; o byte do arquivo tem de ser igual ao da memória
// (senão o valor foi calculado no carregamento e trocar o arquivo não adianta). Sem candidato provado: o número sai
// do código (constante ou fórmula) ou de tabela toda zero (`.bss`, que não está no arquivo) — "nao_editavel".
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <string>
#include <vector>
#include <unordered_map>

typedef void(__cdecl* FnVoid)();
typedef void*(__cdecl* FnCreate)(unsigned int, int);
typedef int(__cdecl* FnIdNivel)(unsigned int, int);
typedef int(__thiscall* FnInt)(void*);
typedef void(__thiscall* FnDestroy)(void*);

static HMODULE dll;
static unsigned char* base;
static FnCreate criar;
static FnDestroy destruir;
static void* sym(const char* n) { return (void*)GetProcAddress(dll, n); }

struct Secao { DWORD rva, tam, arq, tam_arq; bool gravavel; };
static std::vector<Secao> secoes;

static int slot_unico(void** vt, const char* n) {
    void* f = sym(n);
    if (!f) return -1;
    int achado = -1, vezes = 0;
    for (int i = 0; i < 64; i++) if (vt[i] == f) { achado = i; vezes++; }
    return vezes == 1 ? achado : -1;
}

static long arq_de_rva(DWORD rva) {
    for (auto& s : secoes)
        if (rva >= s.rva && rva < s.rva + s.tam_arq) return (long)(rva - s.rva + s.arq);
    return -1;
}

// Chama a função de leitura protegida: um candidato errado (ponteiro, tabela de outra coisa) pode fazer o código
// do DLL cair; aí o candidato simplesmente não é a tabela.
static FnIdNivel g_est; static int g_slot; static unsigned int g_id; static int g_nivel = 1;
static int ler_bruto() {
    if (g_est) return g_est(g_id, g_nivel);
    if (g_slot < 0) return 0x7fffffff;
    void* s = criar(g_id, g_nivel);
    if (!s) return 0x7fffffff;
    int r = ((FnInt)(*(void***)s)[g_slot])(s);
    destruir(s);
    return r;
}
static int ler_protegido(int* ok) {
    __try { *ok = 1; return ler_bruto(); }
    __except (EXCEPTION_EXECUTE_HANDLER) { *ok = 0; return 0; }
}

int main(int argc, char** argv) {
    if (argc < 3) { fprintf(stderr, "uso: enderecos.exe elementskill.dll entrada.txt\n"); return 2; }
    dll = LoadLibraryA(argv[1]);
    if (!dll) { fprintf(stderr, "LoadLibrary falhou: %lu\n", GetLastError()); return 1; }
    base = (unsigned char*)dll;
    FnVoid init = (FnVoid)sym("?InitStaticData@ElementSkill@GNET@@SAXXZ");
    criar = (FnCreate)sym("?Create@ElementSkill@GNET@@SAPAV12@IH@Z");
    destruir = (FnDestroy)sym("?Destroy@ElementSkill@GNET@@QAEXXZ");
    void** vt = (void**)sym("??_7ElementSkill@GNET@@6B@");
    if (!criar || !destruir || !vt) return 1;
    if (init) init();

    // O arquivo, para conferir byte a byte.
    FILE* fa = fopen(argv[1], "rb");
    fseek(fa, 0, SEEK_END);
    long tam_arquivo = ftell(fa);
    fseek(fa, 0, SEEK_SET);
    std::vector<unsigned char> arquivo(tam_arquivo);
    fread(arquivo.data(), 1, tam_arquivo, fa);
    fclose(fa);

    // Seções de dados inicializados e graváveis/só-leitura (não código).
    IMAGE_NT_HEADERS* nt = (IMAGE_NT_HEADERS*)(base + ((IMAGE_DOS_HEADER*)base)->e_lfanew);
    IMAGE_SECTION_HEADER* sh = IMAGE_FIRST_SECTION(nt);
    std::unordered_map<unsigned int, std::vector<DWORD>> indice;  // valor int32 → RVAs alinhados
    for (int i = 0; i < nt->FileHeader.NumberOfSections; i++) {
        DWORD c = sh[i].Characteristics;
        if (c & IMAGE_SCN_CNT_CODE) continue;
        if (!(c & IMAGE_SCN_CNT_INITIALIZED_DATA)) continue;
        if (!memcmp(sh[i].Name, ".reloc", 6)) continue;  // relocações, não dados
        Secao s = {sh[i].VirtualAddress, sh[i].Misc.VirtualSize, sh[i].PointerToRawData, sh[i].SizeOfRawData, (c & IMAGE_SCN_MEM_WRITE) != 0};
        if (s.tam_arq > s.tam) s.tam_arq = s.tam;
        secoes.push_back(s);
        fprintf(stderr, "seção %.8s rva %08lx tam %lu (no arquivo %lu)\n", sh[i].Name, s.rva, s.tam, s.tam_arq);
        for (DWORD r = s.rva; r + 4 <= s.rva + s.tam_arq; r += 4) indice[*(unsigned int*)(base + r)].push_back(r);
    }

    int imp = slot_unico(vt, "?GetMpCost@ElementSkill@GNET@@UAEHXZ");
    int irec = slot_unico(vt, "?GetCoolingTime@ElementSkill@GNET@@UAEHXZ");
    struct { const char* campo; const char* estatica; int* slot; } acesso[] = {
        {"nivel_exigido", "?GetRequiredLevel@ElementSkill@GNET@@SAHIH@Z", nullptr},
        {"sp_exigido", "?GetRequiredSp@ElementSkill@GNET@@SAHIH@Z", nullptr},
        {"dinheiro_exigido", "?GetRequiredMoney@ElementSkill@GNET@@SAHIH@Z", nullptr},
        {"item_exigido", "?GetRequiredBook@ElementSkill@GNET@@SAHIH@Z", nullptr},
        {"execucao_ms", "?GetExecuteTime@ElementSkill@GNET@@SAHIH@Z", nullptr},
        {"mp", nullptr, &imp},
        {"recarga_ms", nullptr, &irec},
    };
    fprintf(stderr, "slots: mp %d, recarga %d\n", imp, irec);

    FILE* fe = fopen(argv[2], "r");
    if (!fe) return 1;
    char campo[64], tipo[4];
    unsigned int id;
    int n, achados = 0, sem = 0, linhas = 0;
    while (fscanf(fe, "%u %63s %3s %d", &id, campo, tipo, &n) == 4) {
        std::vector<unsigned int> seq(n);
        for (int k = 0; k < n; k++) {
            if (tipo[0] == 'f') { float x; fscanf(fe, "%f", &x); memcpy(&seq[k], &x, 4); }
            else { int x; fscanf(fe, "%d", &x); seq[k] = (unsigned int)x; }
        }
        linhas++;
        // A função que lê o número no nível 1.
        FnIdNivel est = nullptr; int slot = -1;
        for (auto& a : acesso)
            if (!strcmp(a.campo, campo)) { if (a.estatica) est = (FnIdNivel)sym(a.estatica); else slot = *a.slot; }
        g_est = est; g_slot = slot; g_id = id;
        auto ler = [&](int* ok) -> int { return ler_protegido(ok); };
        // O elemento mais raro dá os candidatos.
        int melhor = -1; size_t menos = (size_t)-1;
        for (int k = 0; k < n; k++) {
            auto it = indice.find(seq[k]);
            size_t q = it == indice.end() ? 0 : it->second.size();
            if (q < menos) { menos = q; melhor = k; }
        }
        std::vector<DWORD> provados;
        const char* motivo = "sem tabela nos dados";
        bool todo_zero = true;
        for (int k = 0; k < n; k++) if (seq[k] && !(tipo[0] == 'f' && seq[k] == 0x80000000u)) todo_zero = false;
        std::vector<DWORD> iguais;
        if (todo_zero) motivo = "tudo zero: nao procurada";
        else if (melhor >= 0 && menos > 0) {
            for (DWORD r : indice[seq[melhor]]) {
                if (r < (DWORD)melhor * 4) continue;
                DWORD ini = r - melhor * 4;
                if (!memcmp(base + ini, seq.data(), n * 4)) iguais.push_back(ini);
            }
            if (iguais.size() > 5000) { motivo = "ambigua: tabelas iguais demais"; iguais.clear(); }
        }
        if (!iguais.empty()) {
            // Linha de base por nível.
            std::vector<int> antes(n);
            bool ok_antes = true;
            for (int k = 0; k < n; k++) { int ok; g_nivel = k + 1; antes[k] = ler(&ok); ok_antes = ok_antes && ok; }
            for (DWORD ini : iguais) {
                // B305: só seção gravável — em `.rdata` o compilador funde constantes iguais (o mesmo `float` serve
                // a várias funções), e trocar o byte muda o que não se queria (o DLL caiu na prova da 121).
                bool gravavel = false;
                for (auto& sc : secoes) if (ini >= sc.rva && ini < sc.rva + sc.tam) gravavel = sc.gravavel;
                if (!gravavel) { motivo = "so em secao somente-leitura (constante possivelmente compartilhada)"; continue; }
                // Sentinela em cada nível, lida naquele nível: todos têm de responder.
                bool todos = ok_antes;
                for (int k = 0; k < n && todos; k++) {
                    unsigned int* p = (unsigned int*)(base + ini) + k;
                    DWORD velho;
                    VirtualProtect(p, 4, PAGE_READWRITE, &velho);
                    unsigned int guardado = *p;
                    if (tipo[0] == 'f') { float x; memcpy(&x, p, 4); x += 7777.0f; memcpy(p, &x, 4); }
                    else *p = guardado + 7777;
                    int ok;
                    g_nivel = k + 1;
                    int depois = ler(&ok);
                    *p = guardado;
                    VirtualProtect(p, 4, velho, &velho);
                    if (!ok || depois == antes[k]) todos = false;
                }
                g_nivel = 1;
                if (todos) provados.push_back(ini);
                else if (strcmp(motivo, "sem tabela nos dados")) {}
            }
            if (provados.size() > 1) motivo = "mais de uma tabela";
        }
        // Sem tabela no arquivo: procurar na parte de `.data` que só existe na memória (a função já foi chamada acima,
        // então uma tabela que o código preenche na primeira chamada já está lá).
        if (provados.empty() && !todo_zero) {
            int ok0; ler(&ok0);
            for (auto& sc : secoes) {
                if (sc.tam <= sc.tam_arq) continue;
                for (DWORD r = sc.rva + sc.tam_arq; r + n * 4 <= sc.rva + sc.tam; r += 4)
                    if (!memcmp(base + r, seq.data(), n * 4)) { motivo = "tabela preenchida pelo codigo no carregamento (fora do arquivo)"; break; }
            }
        }
        printf("{\"id\":%u,\"campo\":\"%s\",\"tipo\":\"%s\",\"n\":%d", id, campo, tipo, n);
        if (provados.size() == 1) {
            long off = arq_de_rva(provados[0]);
            bool igual = off >= 0 && off + n * 4 <= tam_arquivo && !memcmp(arquivo.data() + off, base + provados[0], n * 4);
            printf(",\"rva\":%lu,\"arquivo\":%ld,\"igual_ao_arquivo\":%s}\n", provados[0], off, igual ? "true" : "false");
            if (igual) achados++; else sem++;
        } else {
            printf(",\"nao_editavel\":\"%s\",\"candidatos_provados\":%zu}\n", motivo, provados.size());
            sem++;
        }
    }
    fprintf(stderr, "linhas %d: no arquivo %d, não editáveis %d\n", linhas, achados, sem);
    return 0;
}
