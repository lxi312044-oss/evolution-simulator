"""Validate Phase 06.2.43 carnivore lineage using full (index,generation) IDs."""
import argparse,csv,io,json,zipfile
from pathlib import Path

def verify(rows):
    nodes={}; errors=[]
    for row in rows:
        key=(int(row['id_index']),int(row['id_generation']))
        if key in nodes: errors.append(f'duplicate entity {key}')
        nodes[key]=row
    roots=0; missing=0
    for key,row in nodes.items():
        pi=row['parent_id_index'];pg=row['parent_id_generation']
        if not pi and not pg: roots+=1;continue
        if not pi or not pg: errors.append(f'partial parent ID for {key}');continue
        parent=(int(pi),int(pg))
        if parent not in nodes:missing+=1;errors.append(f'missing parent {parent} for {key}');continue
        if int(nodes[parent]['tick']) < int(row['tick']) and nodes[parent]['event']=='death':
            # Parent can die after child's birth; death snapshot time cannot determine birth order.
            pass
    for start in nodes:
        seen=set();cur=start
        while cur in nodes:
            if cur in seen:errors.append(f'cycle at {cur} from {start}');break
            seen.add(cur);row=nodes[cur]
            if not row['parent_id_index'] or not row['parent_id_generation']:break
            cur=(int(row['parent_id_index']),int(row['parent_id_generation']))
    return {'records':len(rows),'roots':roots,'missing_parents':missing,'survivors':sum(r['event']!='death' for r in rows),'errors':errors}

def main():
    p=argparse.ArgumentParser();p.add_argument('zipfile');args=p.parse_args()
    with zipfile.ZipFile(args.zipfile) as z:
        files=sorted(x for x in z.namelist() if x.endswith('.life.csv'))
        if len(files)!=5:raise SystemExit(f'Expected 5 genetic life CSVs, got {len(files)}')
        for f in files:
            rows=list(csv.DictReader(io.StringIO(z.read(f).decode('utf-8-sig'))))
            result=verify(rows);print(f, json.dumps(result,ensure_ascii=False))
            if result['errors']:raise SystemExit('Lineage validation failed')
if __name__=='__main__':main()
