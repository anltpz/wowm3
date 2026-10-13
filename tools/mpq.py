import struct, sys, zlib
def mk_table():
    t=[0]*0x500; seed=0x00100001
    for i in range(0x100):
        idx=i
        for j in range(5):
            seed=(seed*125+3)%0x2AAAAB; a=(seed&0xFFFF)<<16
            seed=(seed*125+3)%0x2AAAAB; b=seed&0xFFFF
            t[idx]=a|b; idx+=0x100
    return t
T=mk_table()
def h(s,ht):
    s1=0x7FED7FED; s2=0xEEEEEEEE
    for c in s.upper().replace(chr(47),chr(92)).encode():
        s1=(T[ht*0x100+c]^(s1+s2))&0xFFFFFFFF
        s2=(c+s1+s2+(s2<<5)+3)&0xFFFFFFFF
    return s1
def dec(data,key):
    out=bytearray(); s2=0xEEEEEEEE
    for i in range(0,len(data)-len(data)%4,4):
        s2=(s2+T[0x400+(key&0xFF)])&0xFFFFFFFF
        v=struct.unpack_from('<I',data,i)[0]^((key+s2)&0xFFFFFFFF)
        key=((((~key)<<0x15)+0x11111111)|(key>>0x0B))&0xFFFFFFFF
        s2=(v+s2+(s2<<5)+3)&0xFFFFFFFF
        out+=struct.pack('<I',v)
    return bytes(out)
class MPQ:
    def __init__(s,p):
        s.f=open(p,'rb'); hd=s.f.read(32)
        assert hd[:4]==b'MPQ\x1a'
        _,hs,_,fv,ss,hto,bto,hn,bn=struct.unpack('<4sIIHHIIII',hd)
        s.sec=512<<ss
        s.f.seek(hto); s.ht=dec(s.f.read(hn*16),h('(hash table)',3)); s.hn=hn
        s.f.seek(bto); s.bt=dec(s.f.read(bn*16),h('(block table)',3))
    def find(s,name):
        i=h(name,0)%s.hn; a=h(name,1); b=h(name,2)
        while True:
            ha,hb,lp,bi=struct.unpack_from('<IIII',s.ht,i*16)
            if bi==0xFFFFFFFF: return None
            if ha==a and hb==b: return bi
            i=(i+1)%s.hn
    def read(s,name):
        bi=s.find(name)
        if bi is None: return None
        off,csz,fsz,fl=struct.unpack_from('<IIII',s.bt,bi*16)
        s.f.seek(off); raw=s.f.read(csz)
        if fl&0x10000: raise Exception('encrypted')
        if fl&0x200 and csz!=fsz:  # compressed sectors
            n=(fsz+s.sec-1)//s.sec; offs=struct.unpack_from('<%dI'%(n+1),raw,0); out=b''
            for k in range(n):
                c=raw[offs[k]:offs[k+1]]; want=min(s.sec,fsz-k*s.sec)
                if len(c)<want:
                    m=c[0]; c=c[1:]
                    if m==2: c=zlib.decompress(c)
                    else: raise Exception('comp %x'%m)
                out+=c
            return out
        return raw[:fsz]
if __name__=='__main__':
    m=MPQ(sys.argv[1]); lf=m.read('(listfile)').decode('latin1').split()
    open(sys.argv[2],'w').write('\n'.join(lf)); print(len(lf))
