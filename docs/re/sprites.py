# sprites.py RAW2D [scan] — the 2D sprite ids (FRONTEND.md 1): id -> TEKKEN.BIN file, upload, exported texture
# (textures/FFF_NNN.png, NNN = image uploads counted without palettes), size. "scan": every constant sprite id the
# code draws (calls of the 0x3CE2xx-0x3CE5xx primitives with a1 / a2 constants), with the caller.
import struct, sys, os, re
V = 0x33F000
HERE = sys.path[0]
elf = open(HERE + '/disc/SLUS_200.01', 'rb').read()
FIRST = struct.unpack_from('<53i', elf, 0x520990 - V)          # first sprite id of each group
RES = struct.unpack_from('<52h', elf, 0x4FBF50 - V)            # group (resource id) -> TEKKEN.BIN file (0 = loaded at run time)
def uploads(f):
    d = open(os.path.join(sys.argv[1], '%03d.bin' % f), 'rb').read(); o = 0; out = []
    while o + 16 <= len(d):
        lo = struct.unpack_from('<Q', d, o)[0]; nloop = lo & 0x7FFF; nreg = (lo >> 60) or 16
        if nloop == 0: break
        if (lo >> 58) & 3 == 0:
            r = {}
            for k in range(nloop * nreg):
                v, a = struct.unpack_from('<QQ', d, o + 16 + 16 * k); r[a & 0xFF] = v
            q = o + 16 + 16 * nloop * nreg; n = struct.unpack_from('<Q', d, q)[0] & 0x7FFF
            g = r.get(0x52, 0); psm = (r.get(0x50, 0) >> 56) & 0x3F; w, h = g & 0xFFF, (g >> 32) & 0xFFF
            out.append((w, h, psm == 0 and (w, h) in ((8, 2), (16, 16))))
            o = q + 16 + 16 * n
        else: o += 16 + 16 * nloop
    return out
cache = {}
def sprite(i):
    g = max(k for k in range(52) if FIRST[k] <= i); f = RES[g]; u = i - FIRST[g]
    if f == 0: return g, f, u, '(run time)', ''
    ups = cache.setdefault(f, uploads(f))
    if u >= len(ups): return g, f, u, '?', ''
    w, h, pal = ups[u]
    return g, f, u, 'palette' if pal else '%03d_%03d.png' % (f, sum(1 for x in ups[:u] if not x[2])), '%dx%d' % (w, h)
if len(sys.argv) > 2 and sys.argv[2] == 'scan':
    L = open(HERE + '/out/code.asm').read().split('\n')
    prim = {'3ce258', '3ce2b8', '3ce318', '3ce398', '3ce418', '3ce508', '3ce5d8', '3cd9f8'}
    for i, l in enumerate(L):
        m = re.search(r'jal +0x(\w+)$', l)
        if not m or m.group(1) not in prim: continue
        regs = {}
        for k in range(max(0, i - 30), i + 2):
            mm = re.search(r'(addiu|ori) +\$(a1|a2|a3|t0|t1|t2), \$zero, (-?0x[0-9a-f]+|-?\d+)$', L[k])
            if mm: regs[mm.group(2)] = int(mm.group(3), 0)
        if 'a1' in regs:
            s = regs['a1'] if m.group(1) != '3cd9f8' else regs.get('a0', regs['a1'])
            print(l[:8], m.group(1), 'id %3d' % s, *sprite(s), {k: v for k, v in regs.items() if k != 'a1'})
else:
    for i in range(FIRST[51] + 2):
        print(i, *sprite(i))
