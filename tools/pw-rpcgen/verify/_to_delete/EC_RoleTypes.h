#pragma once
// Tipos que o cabeçalho usa mas declara em outro lugar do cliente. Os tamanhos aqui
// são arbitrários de propósito: nenhuma struct que os contenha recebe tamanho no IR,
// então nenhuma asserção depende deles.
struct ROLEEXTPROP_BASE { int _pad[8]; };
struct ROLEEXTPROP_MOVE { int _pad[8]; };
struct ROLEEXTPROP_ATK  { int _pad[8]; };
struct ROLEEXTPROP_DEF  { int _pad[8]; };
struct ROLEEXTPROP      { int _pad[32]; };

// Símbolos que o cabeçalho usa mas não declara. Só `NUM_PROFESSION` aparece como
// comprimento de array (`int ranks_size[NUM_PROFESSION+1]`); o pw-rpcgen marca esse
// campo como não resolvido e a struct fica sem tamanho, então nenhuma asserção
// depende do valor escolhido aqui.
#define NULL 0
enum { GENDER_MALE = 0, GENDER_FEMALE = 1 };
enum { NUM_PROFESSION = 8 };
