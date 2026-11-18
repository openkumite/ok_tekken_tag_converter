# xr.py ADDR [ADDR...] — code that references each address (from xref.pkl, built by xref.py)
import pickle, sys
x = pickle.load(open(sys.path[0] + '/xref.pkl', 'rb'))
for a in sys.argv[1:]:
    a = int(a, 16); print('%08x:' % a, ' '.join('%x' % p for p in x.get(a, [])))
