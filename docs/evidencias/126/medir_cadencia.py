"""PCAP clássico Ethernet/IPv4/TCP; tempo de recepção completa do frame GNET.
Uso: python medir_cadencia.py full_interno.pcap. Saída ao lado deste script.
Remonta por sequência, rejeita buracos, ignora retransmissões já cobertas.
Complementa pw-pcapdiff (que não preserva timestamps); não mede animação na tela.
"""
import sys, struct, bisect, json, collections, hashlib
from pathlib import Path
raw=Path(sys.argv[1]).read_bytes()
assert raw[:4]==bytes.fromhex('d4c3b2a1'), 'exige PCAP LE em microssegundos'
assert struct.unpack_from('<I',raw,20)[0]==1, 'exige Ethernet'
streams=collections.defaultdict(list); pos=24; timestamps=[]
while pos<len(raw):
 sec,usec,n,orig=struct.unpack_from('<IIII',raw,pos);pos+=16
 frame=raw[pos:pos+n];pos+=n;assert n==orig and len(frame)==n
 t=sec+usec/1e6;timestamps.append(t)
 if frame[12:14]!=b'\x08\x00':continue
 ip=frame[14:]; ihl=(ip[0]&15)*4
 if ip[9]!=6:continue
 assert struct.unpack_from('>H',ip,6)[0]&0x3fff==0, 'IP fragmentado'
 tcp=ip[ihl:struct.unpack_from('>H',ip,2)[0]]
 sp,dp,seq=struct.unpack_from('>HHI',tcp);h=(tcp[12]>>4)*4;data=tcp[h:]
 if 29301 not in (sp,dp) or not data:continue
 streams[(ip[12:16].hex(),sp,ip[16:20].hex(),dp)].append((seq,t,data))
def cu(b,p):
 v=b[p];p+=1
 if v<128:return v,p
 if v<192:return ((v&63)<<8)|b[p],p+1
 if v<224:return ((v&31)<<24)|int.from_bytes(b[p:p+3],'big'),p+3
 return int.from_bytes(b[p:p+4],'big'),p+4
t0=min(timestamps)
records=[];counts=collections.Counter();frame_counts=[]
for key,segs in streams.items():
 segs.sort(key=lambda s:(s[0],s[1]));base=segs[0][0];buf=bytearray();ends=[];times=[];last=0
 for seq,t,data in segs:
  start=seq-base;assert start<=len(buf), 'buraco TCP'
  overlap=len(buf)-start
  if overlap>=len(data):continue
  assert buf[start:]==data[:overlap], 'retransmissão inconsistente'
  buf.extend(data[overlap:]);ends.append(len(buf));last=max(last,t);times.append(last)
 p=0;frames=0
 while p<len(buf):
  op,p=cu(buf,p);n,p=cu(buf,p);end=p+n;assert end<=len(buf),'frame GNET incompleto'
  t=times[bisect.bisect_left(ends,end)];b=buf[p:end];p=end;frames+=1
  if op not in (74,75,77):continue
  k=8 if op in (74,75) else 0
  size,k=cu(b,k);d=bytes(b[k:k+size]);assert len(d)==size and size>=2
  cmd=int.from_bytes(d[:2],'little');direction='C2S' if op==75 else 'S2C'
  counts[direction,cmd]+=1
  role=int.from_bytes(b[:4],'big') if op in (74,75) else None
  records.append(dict(t=round(t-t0,6),dir=direction,cmd=cmd,role=role,hex=d[2:].hex()))
 frame_counts.append((key,frames))
records=[r for r in records if r['cmd'] in (3,23,24,26,33,38,83,84,144)]
records.sort(key=lambda r:r['t']);out=Path(__file__).parent
(out/'cadencia-eventos.json').write_text(json.dumps(records,indent=1),encoding='utf-8')
lines=['# Cadência original 126','', 'SHA256 PCAP: '+hashlib.sha256(raw).hexdigest(),
 'Tempo relativo ao primeiro quadro; frame completo recebido no elo interno, não tempo de animação.',
 'Fluxos (quadros GNET): '+str(frame_counts),'', '| role | t (s) | comando | dados | intervalo desde resultado anterior (ms) |', '|---|---:|---|---|---:|']
last={};cadences=[]
for r in records:
 if r['dir']!='S2C' or r['cmd'] not in (23,24,26,33,38,83,84):continue
 role=r['role'];c=r['cmd'];b=bytes.fromhex(r['hex']);delta=''
 if c==84:last.pop(role,None)
 if c==24:
  if role in last:delta=round((r['t']-last[role])*1000,3);cadences.append(delta)
  last[role]=r['t']
 if c==23:last.pop(role,None)
 lines.append(f"| {role} | {r['t']:.6f} | {c} | {r['hex']} | {delta} |")
lines+=['','Contagens S2C: '+str({i:counts['S2C',i] for i in (23,24,26,33,38,83,84,144)}), 'Intervalos 24 dentro de sessão (ms): '+str(cadences)]
(out/'cadencia-original.md').write_text('\n'.join(lines)+'\n',encoding='utf-8')
print(lines[-2]);print(lines[-1])
