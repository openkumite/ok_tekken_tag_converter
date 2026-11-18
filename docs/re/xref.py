# Address cross-references: lui + (addiu|ori|load|store) pairs in the code. Writes xref.pkl {target: [pc..]}.
import struct, pickle, sys, collections
V = 0x33f000
d = open(sys.path[0] + '/disc/SLUS_200.01', 'rb').read()
TEXT_END = 0x420000 - V  # code is 0x340000..0x41ffff
def build():
    x = collections.defaultdict(list); lui = {}
    for o in range(0x1000, TEXT_END, 4):
        w, = struct.unpack_from('<I', d, o); op = w >> 26; rs = (w >> 21) & 31; rt = (w >> 16) & 31; imm = w & 0xffff
        simm = imm - 0x10000 if imm & 0x8000 else imm; pc = o + V
        if op == 0x0f: lui[rt] = (imm << 16, pc); continue
        if rs in lui and pc - lui[rs][1] < 0x100:
            if op == 9 or op == 0x19: x[(lui[rs][0] + simm) & 0xffffffff].append(pc); 
            elif op == 0x0d: x[lui[rs][0] | imm].append(pc)
            elif op >= 0x20 and op not in (0x2f, 0x33): x[(lui[rs][0] + simm) & 0xffffffff].append(pc)
        if op in (9, 0x0d, 0x19) and rt != rs: lui.pop(rt, None)
        if 0x20 <= op <= 0x27 or op == 0x37: lui.pop(rt, None)
        if op == 0:
            rd = (w >> 11) & 31
            if w & 0x3f not in (8,): lui.pop(rd, None)
    return dict(x)
if __name__ == '__main__':
    x = build(); pickle.dump(x, open(sys.path[0] + '/xref.pkl', 'wb')); print(len(x), 'targets')
