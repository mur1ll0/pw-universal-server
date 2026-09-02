#ifndef __STUB_TYPES_H__
#define __STUB_TYPES_H__
// Substituto mínimo de cgame/common/types.h para a verificação de tamanhos.
// O types.h original puxa <algorithm> e outros cabeçalhos da libstdc++, que não existem
// em 32 bits neste contêiner e que não descrevem formato de fio nenhum. Aqui ficam só
// os tipos que protocol.h de fato usa no layout.
typedef __SIZE_TYPE__ size_t;
// stdint: o protocol.h usa int64_t/uint64_t em máscaras de equipamento e em dinheiro.
typedef signed char        int8_t;   typedef unsigned char      uint8_t;
typedef short              int16_t;  typedef unsigned short     uint16_t;
typedef int                int32_t;  typedef unsigned int       uint32_t;
typedef long long          int64_t;  typedef unsigned long long uint64_t;
typedef unsigned long DWORD;
typedef unsigned char BYTE;
typedef unsigned short WORD;
#define __int64 long long

#pragma pack(1)
struct A3DVECTOR { float x, y, z; A3DVECTOR() {} A3DVECTOR(float a,float b,float c):x(a),y(b),z(c){} };
namespace S2C {
  struct single_data_header { unsigned short cmd; };
  // 4 bytes: opcode + contagem. As structs de lista abrem com ele.
  struct multi_data_header  { unsigned short cmd; unsigned short count; };
}
namespace C2S { struct cmd_header { unsigned short cmd; }; }
#pragma pack()
#endif
