# vdis.py ADDR [N] — like fn.py (the function containing ADDR, from out/code.asm), but with the EE's VU0 macro
# instructions (COP2, lqc2, sqc2) and sqrt.s decoded; capstone shows those as MSA / .word garbage.
import sys, struct
V = 0x33f000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
L = open(sys.path[0] + '/out/code.asm').read().split('\n')
R = 'zero at v0 v1 a0 a1 a2 a3 t0 t1 t2 t3 t4 t5 t6 t7 s0 s1 s2 s3 s4 s5 s6 s7 t8 t9 k0 k1 gp sp fp ra'.split()
BC = 'xyzw'
S1 = {0x1C: 'vmulq', 0x1D: 'vmaxi', 0x1E: 'vmuli', 0x1F: 'vminii', 0x20: 'vaddq', 0x21: 'vmaddq', 0x22: 'vaddi',
      0x23: 'vmaddi', 0x24: 'vsubq', 0x25: 'vmsubq', 0x26: 'vsubi', 0x27: 'vmsubi', 0x28: 'vadd', 0x29: 'vmadd',
      0x2A: 'vmul', 0x2B: 'vmax', 0x2C: 'vsub', 0x2D: 'vmsub', 0x2E: 'vopmsub', 0x2F: 'vmini', 0x30: 'viadd',
      0x31: 'visub', 0x32: 'viaddi', 0x34: 'viand', 0x35: 'vior', 0x38: 'vcallms', 0x39: 'vcallmsr'}
S1BC = ['vadd', 'vsub', 'vmadd', 'vmsub', 'vmax', 'vmini', 'vmul']
S2 = {0x1C: 'vmulaq', 0x1D: 'vabs', 0x1E: 'vmulai', 0x1F: 'vclipw', 0x20: 'vaddaq', 0x21: 'vmaddaq', 0x22: 'vaddai',
      0x23: 'vmaddai', 0x24: 'vsubaq', 0x25: 'vmsubaq', 0x26: 'vsubai', 0x27: 'vmsubai', 0x28: 'vadda', 0x29: 'vmadda',
      0x2A: 'vmula', 0x2C: 'vsuba', 0x2D: 'vmsuba', 0x2E: 'vopmula', 0x2F: 'vnop', 0x30: 'vmove', 0x31: 'vmr32',
      0x34: 'vlqi', 0x35: 'vsqi', 0x36: 'vlqd', 0x37: 'vsqd', 0x38: 'vdiv', 0x39: 'vsqrt', 0x3A: 'vrsqrt',
      0x3B: 'vwaitq', 0x3C: 'vmtir', 0x3D: 'vmfir', 0x40: 'vrnext', 0x41: 'vrget', 0x42: 'vrinit', 0x43: 'vrxor'}
S2BC = ['vadda', 'vsuba', 'vmadda', 'vmsuba', 'vitof', 'vftoi', 'vmula']
def dec(w):
    op = w >> 26
    if op in (0x1E, 0x1F):
        imm = w & 0xffff; imm -= 0x10000 * (imm >> 15)
        return '%s       $%s, %d($%s)' % ('lq' if op == 0x1E else 'sq', R[(w >> 16) & 31], imm, R[(w >> 21) & 31])
    if op in (0x36, 0x3E):
        imm = w & 0xffff; imm -= 0x10000 * (imm >> 15)
        return '%s     $vf%d, %d($%s)' % ('lqc2' if op == 0x36 else 'sqc2', (w >> 16) & 31, imm, R[(w >> 21) & 31])
    if op == 0x11 and (w >> 21) & 31 == 0x10 and w & 63 == 4:
        return 'sqrt.s   $f%d, $f%d' % ((w >> 6) & 31, (w >> 16) & 31)
    if op == 0x11 and (w >> 21) & 31 == 0x10 and w & 63 == 0x16:
        return 'rsqrt.s  $f%d, $f%d, $f%d' % ((w >> 6) & 31, (w >> 11) & 31, (w >> 16) & 31)
    if op != 0x12: return None
    rs, rt, rd = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31
    if not w >> 25 & 1:
        n = {1: 'qmfc2', 2: 'cfc2', 5: 'qmtc2', 6: 'ctc2'}.get(rs, 'cop2_%d' % rs)
        return '%-8s $%s, $vf%d' % (n, R[rt], rd) if n in ('qmfc2', 'qmtc2') else '%-8s $%s, $vi%d' % (n, R[rt], rd)
    dest = ''.join(c for i, c in enumerate('xyzw') if w >> (24 - i) & 1)
    ft, fs, fd, fn = rt, rd, (w >> 6) & 31, w & 63
    if fn < 0x3C:
        if fn < 0x1C: n = S1BC[fn >> 2] + BC[fn & 3]; return '%s.%s $vf%d, $vf%d, $vf%d%s' % (n, dest, fd, fs, ft, BC[fn & 3])
        n = S1.get(fn, 'vop%x' % fn)
        if fn >= 0x30: return '%s $vi%d, $vi%d, $vi%d' % (n, fd, fs, ft)
        return '%s.%s $vf%d, $vf%d, $vf%d' % (n, dest, fd, fs, ft)
    c = (fd << 2) | (fn & 3)
    if c < 0x1C:
        g = c >> 2
        if g in (4, 5): return '%s%d.%s $vf%d, $vf%d' % (S2BC[g], [0, 4, 12, 15][c & 3], dest, ft, fs)
        return '%s%s.%s ACC, $vf%d, $vf%d%s' % (S2BC[g], BC[c & 3], dest, fs, ft, BC[c & 3])
    n = S2.get(c, 'v2op%x' % c)
    if n in ('vdiv', 'vrsqrt'): return '%s Q, $vf%d%s, $vf%d%s' % (n, fs, BC[(w >> 21) & 3], ft, BC[(w >> 23) & 3])
    if n == 'vsqrt': return 'vsqrt Q, $vf%d%s' % (ft, BC[(w >> 23) & 3])
    return '%s.%s $vf%d, $vf%d' % (n, dest, ft, fs)
a = int(sys.argv[1], 16); i = (a - 0x340000) // 4
s = i
while s > 0 and not L[s - 2].endswith('jr       $ra'): s -= 1
e = i
while not L[e].endswith('jr       $ra'): e += 1
mx = int(sys.argv[2]) if len(sys.argv) > 2 else 400
for k in range(s, min(e + 2, s + mx)):
    pc = 0x340000 + 4 * k; w = struct.unpack_from('<I', d, pc - V)[0]; x = dec(w)
    print('%x  %s' % (pc, x) if x else L[k][2:])
