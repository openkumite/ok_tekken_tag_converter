# aifn.py ADDR [ADDR...] -- print each AI function from ADDR up to the next function start (a jal target, or a
# handler from table 0x507A80), from out/code.asm; calls are labelled with section 19-21's names.
import sys, re
L = open(sys.path[0] + '/out/code.asm').read().split('\n')
starts = set(int(t, 16) for t in re.findall(r'jal +0x([0-9a-f]+)', '\n'.join(L)))
starts |= {0x38B698, 0x38B368, 0x38B820, 0x38B718, 0x38AFC0, 0x38BC00, 0x38BE78}
NAMES = {0x3800D8: 'randint', 0x40FBD0: 'rand', 0x381278: 'cmd_to_pad', 0x383B88: 'in_range', 0x383D70: 'mark',
         0x3853E8: 'pick', 0x385250: 'after_attack', 0x383D10: 'build?', 0x36CF30: 'motion_seq', 0x3811E8: 'play_seq'}
for a in map(lambda s: int(s, 16), sys.argv[1:]):
    end = min([s for s in starts if s > a] + [a + 0x4000])
    for x in L[(a - 0x340000) // 4:(end - 0x340000) // 4]:
        m = re.search(r'jal +0x([0-9a-f]+)', x)
        print(x + ('   ; ' + NAMES[int(m.group(1), 16)] if m and int(m.group(1), 16) in NAMES else ''))
    print('----')
