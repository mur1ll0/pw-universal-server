/* Estúdio, fase S5 (B272): modelos 3D. Lista os `.ski` do cliente publicado, mostra a ficha (ossos, texturas, materiais,
   malhas) e desenha o GLB que o pw-recursos gera (`pw-data-loader/src/glb.rs`) num visualizador WebGL próprio — sem
   biblioteca externa (CSP `self`). Textura (B273: PNG embutido no GLB, alfa por corte) e luz direcional; sem esqueleto.
   Usa `api`, `criar`, `elemento`, `alerta`, `estudio` e `mostrarVistaDoEstudio`. */

function rotaDeModelos(resto, q = {}) {
  const p = new URLSearchParams(q).toString();
  return `/api/realms/${encodeURIComponent(estudio.realm)}/estudio/modelos${resto}${p ? `?${p}` : ""}`;
}

async function carregarModelos() {
  if (!estudio.realm) return;
  elemento("modelos-situacao").textContent = "Carregando…";
  let r;
  try { r = await api(rotaDeModelos("", { busca: elemento("modelos-busca").value.trim() })); }
  catch (erro) { elemento("modelos-situacao").textContent = erro.message; return; }
  elemento("modelos-situacao").textContent = `${r.total} .ski${r.total > r.linhas.length ? ` (${r.linhas.length} primeiros)` : ""}`;
  const area = elemento("modelos-lista");
  area.replaceChildren();
  for (const l of r.linhas) {
    const b = criar("button", "", l.caminho.replace(/^models\\/i, ""));
    b.type = "button";
    b.title = l.caminho;
    b.addEventListener("click", () => abrirModelo(l.caminho));
    area.appendChild(b);
  }
  if (!r.linhas.length) area.appendChild(criar("div", "texto-suave", "Nenhum modelo."));
}

async function abrirModelo(caminho) {
  const ficha = elemento("modelos-ficha");
  ficha.textContent = "Carregando…";
  let f;
  try { f = await api(rotaDeModelos("/ficha", { caminho })); } catch (erro) { ficha.textContent = erro.message; return; }
  ficha.replaceChildren();
  ficha.appendChild(criar("h3", "", caminho.split("\\").pop()));
  ficha.appendChild(criar("div", "texto-suave", `${caminho} · versão ${f.versao} · ${f.malhas.length} malhas · ${f.ossos.length} ossos na pele (esqueleto: ${f.ossos_do_esqueleto})`));
  const tela = criar("canvas", "modelo-tela");
  tela.width = 640; tela.height = 480;
  tela.style.width = "100%"; tela.style.maxWidth = "640px"; tela.style.background = "#20242a"; tela.style.borderRadius = "6px";
  ficha.appendChild(tela);
  const baixar = criar("a", "", "Baixar GLB (Blender)");
  baixar.href = rotaDeModelos("/glb", { caminho });
  ficha.appendChild(baixar);
  const lista = criar("ul");
  for (const m of f.malhas) lista.appendChild(criar("li", "", `${m.nome || "(sem nome)"} — ${m.tipo}, ${m.vertices} vértices, ${m.triangulos} triângulos, textura ${f.texturas[m.textura] ?? m.textura}, material ${f.materiais[m.material]?.nome ?? m.material}`));
  ficha.appendChild(lista);
  if (f.texturas.length) {
    const faltam = f.texturas.filter((_, i) => !(f.texturas_achadas || [])[i]);
    ficha.appendChild(criar("div", "texto-suave", `Texturas: ${f.texturas.join(", ")}${faltam.length ? ` — não achadas no cliente (nem em Textures\\ nem em Tex_<nome>\\): ${faltam.join(", ")}` : ""}.`));
  }
  try {
    const resp = await fetch(rotaDeModelos("/glb", { caminho }), { credentials: "same-origin" });
    if (!resp.ok) throw new Error(`GLB: HTTP ${resp.status}`);
    desenharGlb(tela, await resp.arrayBuffer());
  } catch (erro) { alerta("erro", "Estúdio", erro.message); }
}

/* ---- visualizador WebGL mínimo: GLB → buffers → Lambert + órbita ---- */

function lerGlb(buf) {
  const dv = new DataView(buf);
  if (dv.getUint32(0, true) !== 0x46546c67) throw new Error("não é GLB");
  const nj = dv.getUint32(12, true);
  const doc = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 20, nj)));
  const bin = 20 + nj + 8;
  const acessor = (i) => {
    const a = doc.accessors[i], v = doc.bufferViews[a.bufferView];
    const n = { SCALAR: 1, VEC2: 2, VEC3: 3 }[a.type] * a.count;
    return a.componentType === 5126 ? new Float32Array(buf, bin + v.byteOffset, n) : new Uint16Array(buf, bin + v.byteOffset, n);
  };
  // Imagens PNG embutidas (B273): bytes do bufferView.
  const imagens = (doc.images || []).map((im) => { const v = doc.bufferViews[im.bufferView]; return new Blob([new Uint8Array(buf, bin + v.byteOffset, v.byteLength)], { type: im.mimeType }); });
  const malhas = [];
  for (const m of doc.meshes) for (const p of m.primitives) {
    const mat = p.material !== undefined ? doc.materials[p.material] : null;
    const tex = mat?.pbrMetallicRoughness?.baseColorTexture;
    malhas.push({ pos: acessor(p.attributes.POSITION), nor: acessor(p.attributes.NORMAL), uv: acessor(p.attributes.TEXCOORD_0), idx: acessor(p.indices),
      cor: mat ? mat.pbrMetallicRoughness.baseColorFactor : [0.75, 0.75, 0.75, 1], min: doc.accessors[p.attributes.POSITION].min, max: doc.accessors[p.attributes.POSITION].max,
      imagem: tex ? imagens[doc.textures[tex.index].source] : null, corte: mat?.alphaMode === "MASK" ? (mat.alphaCutoff ?? 0.5) : -1 });
  }
  return malhas;
}

function desenharGlb(tela, buf) {
  const gl = tela.getContext("webgl");
  if (!gl) { tela.replaceWith(criar("div", "texto-suave", "WebGL indisponível neste navegador.")); return; }
  const malhas = lerGlb(buf);
  const sh = (tipo, fonte) => { const s = gl.createShader(tipo); gl.shaderSource(s, fonte); gl.compileShader(s); return s; };
  const prog = gl.createProgram();
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, "attribute vec3 p; attribute vec3 n; attribute vec2 t; uniform mat4 mvp; uniform mat4 mdl; varying vec3 vn; varying vec2 vt;" +
    "void main(){ vn = mat3(mdl) * n; vt = t; gl_Position = mvp * vec4(p, 1.0); }"));
  gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, "precision mediump float; varying vec3 vn; varying vec2 vt; uniform vec4 cor; uniform sampler2D img; uniform float usa; uniform float corte;" +
    "void main(){ vec4 c = usa > 0.5 ? texture2D(img, vt) : vec4(1.0); if (corte >= 0.0 && c.a < corte) discard;" +
    " float l = 0.35 + 0.65 * max(dot(normalize(vn), normalize(vec3(0.4, 0.8, 0.5))), 0.0); gl_FragColor = vec4(cor.rgb * c.rgb * l, 1.0); }"));
  gl.linkProgram(prog);
  gl.useProgram(prog);
  const loc = { p: gl.getAttribLocation(prog, "p"), n: gl.getAttribLocation(prog, "n"), t: gl.getAttribLocation(prog, "t"), mvp: gl.getUniformLocation(prog, "mvp"), mdl: gl.getUniformLocation(prog, "mdl"),
    cor: gl.getUniformLocation(prog, "cor"), usa: gl.getUniformLocation(prog, "usa"), corte: gl.getUniformLocation(prog, "corte") };
  const min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity];
  const objs = malhas.map((m) => {
    for (let k = 0; k < 3; k++) { min[k] = Math.min(min[k], m.min[k]); max[k] = Math.max(max[k], m.max[k]); }
    const b = (alvo, dados) => { const x = gl.createBuffer(); gl.bindBuffer(alvo, x); gl.bufferData(alvo, dados, gl.STATIC_DRAW); return x; };
    const o = { pos: b(gl.ARRAY_BUFFER, m.pos), nor: b(gl.ARRAY_BUFFER, m.nor), uv: b(gl.ARRAY_BUFFER, m.uv), idx: b(gl.ELEMENT_ARRAY_BUFFER, m.idx), n: m.idx.length, cor: m.cor, tex: null, corte: m.corte };
    // A textura chega depois (decodificar o PNG é assíncrono); redesenha quando chegar.
    if (m.imagem) createImageBitmap(m.imagem).then((bmp) => {
      const tx = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, tx);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bmp);
      const pot = (x) => (x & (x - 1)) === 0;
      if (pot(bmp.width) && pot(bmp.height)) { gl.generateMipmap(gl.TEXTURE_2D); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR); }
      else { gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); }
      o.tex = tx; desenhar();
    }).catch(() => {});
    return o;
  });
  const centro = [0, 1, 2].map((k) => (min[k] + max[k]) / 2);
  const raio = Math.max(1e-3, Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2);
  const estado = { yaw: 0.6, pitch: 0.3, dist: raio * 2.8 };
  const mul = (a, b) => { const r = new Float32Array(16); for (let i = 0; i < 4; i++) for (let j = 0; j < 4; j++) { let s = 0; for (let k = 0; k < 4; k++) s += a[k * 4 + j] * b[i * 4 + k]; r[i * 4 + j] = s; } return r; };
  const desenhar = () => {
    gl.viewport(0, 0, tela.width, tela.height);
    gl.clearColor(0.125, 0.14, 0.165, 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT); gl.enable(gl.DEPTH_TEST);
    const f = 1 / Math.tan(Math.PI / 8), a = tela.width / tela.height, pe = raio / 50, lo = raio * 50;
    const proj = new Float32Array([f / a, 0, 0, 0, 0, f, 0, 0, 0, 0, (lo + pe) / (pe - lo), -1, 0, 0, (2 * lo * pe) / (pe - lo), 0]);
    const cy = Math.cos(estado.yaw), sy = Math.sin(estado.yaw), cp = Math.cos(estado.pitch), spp = Math.sin(estado.pitch);
    const rot = new Float32Array([cy, spp * sy, -cp * sy, 0, 0, cp, spp, 0, sy, -spp * cy, cp * cy, 0, 0, 0, 0, 1]);
    const tr = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -centro[0], -centro[1], -centro[2], 1]);
    const cam = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, -estado.dist, 1]);
    const mdl = mul(rot, tr);
    const mvp = mul(proj, mul(cam, mdl));
    gl.uniformMatrix4fv(loc.mvp, false, mvp); gl.uniformMatrix4fv(loc.mdl, false, rot);
    for (const o of objs) {
      gl.bindBuffer(gl.ARRAY_BUFFER, o.pos); gl.enableVertexAttribArray(loc.p); gl.vertexAttribPointer(loc.p, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.nor); gl.enableVertexAttribArray(loc.n); gl.vertexAttribPointer(loc.n, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, o.uv); gl.enableVertexAttribArray(loc.t); gl.vertexAttribPointer(loc.t, 2, gl.FLOAT, false, 0, 0);
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, o.tex); gl.uniform1f(loc.usa, o.tex ? 1 : 0); gl.uniform1f(loc.corte, o.corte);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, o.idx); gl.uniform4fv(loc.cor, o.cor);
      gl.drawElements(gl.TRIANGLES, o.n, gl.UNSIGNED_SHORT, 0);
    }
  };
  let arrasto = null;
  tela.addEventListener("mousedown", (e) => { arrasto = [e.clientX, e.clientY]; });
  window.addEventListener("mouseup", () => { arrasto = null; });
  tela.addEventListener("mousemove", (e) => {
    if (!arrasto) return;
    estado.yaw += (e.clientX - arrasto[0]) * 0.01; estado.pitch = Math.max(-1.5, Math.min(1.5, estado.pitch + (e.clientY - arrasto[1]) * 0.01));
    arrasto = [e.clientX, e.clientY]; desenhar();
  });
  tela.addEventListener("wheel", (e) => { e.preventDefault(); estado.dist *= e.deltaY > 0 ? 1.1 : 0.9; desenhar(); }, { passive: false });
  desenhar();
}

(function ligarModelos() {
  elemento("estudio-vista-modelos").addEventListener("click", () => mostrarVistaDoEstudio("modelos"));
  let espera = null;
  elemento("modelos-busca").addEventListener("input", () => { clearTimeout(espera); espera = setTimeout(carregarModelos, 300); });
})();
