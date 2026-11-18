# dis.py ADDR [N]  — disassemble N instructions at memory address ADDR
import sys, capstone
V = 0x33f000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
md = capstone.Cs(capstone.CS_ARCH_MIPS, capstone.CS_MODE_MIPS64 | capstone.CS_MODE_LITTLE_ENDIAN); md.skipdata = True
a = int(sys.argv[1], 16); n = int(sys.argv[2]) if len(sys.argv) > 2 else 40
for i in md.disasm(d[a - V:a - V + 4 * n], a): print('%08x  %-8s %s' % (i.address, i.mnemonic, i.op_str))
