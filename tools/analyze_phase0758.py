import csv,glob,os,sys,math
from collections import defaultdict
root=sys.argv[1] if len(sys.argv)>1 else 'results/phase0758'
files=sorted(glob.glob(os.path.join(root,'*.individuals.csv')))
if len(files)!=6: raise SystemExit(f'Expected 6 individual CSVs, got {len(files)}')
output=[]
for f in files:
    name=os.path.basename(f).split('.')[0]
    groups=defaultdict(list)
    with open(f,encoding='utf-8',newline='') as h:
        for r in csv.DictReader(h):
            if r['event']!='death':continue
            t=int(r['toxin_type']);dose=float(r['lifetime_exposure']);res=float(r['resistance']);births=int(r['successful_births'])
            # Exposed vs unexposed are kept separate; within exposed split by median dose later.
            groups[t].append((dose,res,births,int(r['age'])))
    for t,values in sorted(groups.items()):
        exposed=[v for v in values if v[0]>0]
        for label,subset in [('unexposed',[v for v in values if v[0]==0]),('exposed',exposed)]:
            if not subset:continue
            median=sorted(v[1] for v in subset)[len(subset)//2]
            for group,items in [('low',[v for v in subset if v[1]<median]),('high',[v for v in subset if v[1]>=median])]:
                if not items:continue
                output.append(dict(run=name,toxin_type=t,exposure_group=label,resistance_group=group,n=len(items),mean_resistance=sum(v[1] for v in items)/len(items),mean_exposure=sum(v[0] for v in items)/len(items),mean_births=sum(v[2] for v in items)/len(items),mean_age=sum(v[3] for v in items)/len(items)))
path=os.path.join(root,'selection_strata.csv')
with open(path,'w',encoding='utf-8',newline='') as h:
    w=csv.DictWriter(h,fieldnames=['run','toxin_type','exposure_group','resistance_group','n','mean_resistance','mean_exposure','mean_births','mean_age']);w.writeheader();w.writerows(output)
print('Wrote',path,'rows',len(output))
print('WARNING: observational stratification does not identify causal effects; age/exposure/survivorship confounding remains.')
