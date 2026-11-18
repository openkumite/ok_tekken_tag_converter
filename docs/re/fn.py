# fn.py ADDR — print the function containing ADDR (bounded by 'jr $ra' before and after), from out/code.asm
import sys
L = open(sys.path[0] + '/out/code.asm').read().split('\n')
a = int(sys.argv[1], 16); i = (a - 0x340000) // 4
s = i
while s > 0 and not L[s - 2].endswith('jr       $ra'): s -= 1
e = i
while not L[e].endswith('jr       $ra'): e += 1
mx = int(sys.argv[2]) if len(sys.argv) > 2 else 400
print('\n'.join(x[2:] for x in L[s:min(e + 2, s + mx)]))
