# basefld.py BASE_IMM OFF — instructions using field OFF of a struct whose base was made with "addiu r, r, BASE_IMM" (e.g. -0x4808 for 0x43B7F8) within 120 lines
import re, sys
L = open(sys.path[0] + '/out/code.asm').read().split('\n')
imm, off = sys.argv[1], sys.argv[2]
for i, l in enumerate(L):
    m = re.search(r'addiu +(\$\w+), \$\w+, ' + re.escape(imm) + '$', l)
    if not m: continue
    r = re.escape(m.group(1))
    for j in range(i + 1, min(i + 120, len(L))):
        if re.search(r', ' + re.escape(off) + r'\(' + r + r'\)', L[j]): print(L[j][:60])
        if re.search(r'^\S+ +\w+ +' + r + ',', L[j]) and not re.search(r'^\S+ +(s[bhwd]|sw|beq|bne)', L[j]): break
