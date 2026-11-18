# fulldis.py — whole code segment to out/code.asm (EE-specific opcodes capstone lacks are hand-decoded: mult/madd with rd, MMI)
import sys, struct, capstone
V = 0x33f000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
md = capstone.Cs(capstone.CS_ARCH_MIPS, capstone.CS_MODE_MIPS64 | capstone.CS_MODE_LITTLE_ENDIAN)
R = 'zero at v0 v1 a0 a1 a2 a3 t0 t1 t2 t3 t4 t5 t6 t7 s0 s1 s2 s3 s4 s5 s6 s7 t8 t9 k0 k1 gp sp fp ra'.split()
def ee(w):
    op, rs, rt, rd, fn = w >> 26, (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31, w & 63
    if op == 0 and fn == 0x18 and (w >> 6) & 31 == 0: return 'mult3    $%s, $%s, $%s' % (R[rd], R[rs], R[rt])   # rd = lo = rs*rt
    if op == 0x1c and fn == 0x18: return 'mult1    $%s, $%s, $%s' % (R[rd], R[rs], R[rt])
    if op == 0x1c and fn == 0: return 'madd     $%s, $%s, $%s' % (R[rd], R[rs], R[rt])              # rd = lo += rs*rt
    if op == 0x1c: return 'mmi      %08x' % w
    return None
out = open(sys.path[0] + '/out/code.asm', 'w')
for o in range(0x1000, 0x420000 - V, 4):
    w, = struct.unpack_from('<I', d, o); a = o + V
    s = ee(w)
    if s is None:
        i = next(md.disasm(d[o:o + 4], a), None)
        s = '%-8s %s' % (i.mnemonic, i.op_str) if i else '.word    %08x' % w
    out.write('%08x  %s\n' % (a, s))
