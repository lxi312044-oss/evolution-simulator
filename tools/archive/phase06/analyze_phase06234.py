import csv, pathlib, statistics
root=pathlib.Path(__file__).parent/'results'/'phase06234'
for mode in ('baseline','evasion','high-pressure'):
    rows=[]
    for file in sorted((root/mode).glob('seed-*.csv')):
        with file.open(encoding='utf-8-sig',newline='') as f:
            data=list(csv.DictReader(f))
        if not data: continue
        last=data[-1]
        first=data[0]
        rows.append((file.stem,int(last['tick']),int(last['herbivores']),int(last['carnivores']),float(first['fear_mean']),float(last['fear_mean']),float(first['aggression_mean']),float(last['aggression_mean']),float(last['learned_fear_mean'])))
    print('\n'+mode, 'runs',len(rows))
    for row in rows: print(*row,sep=',')
    if rows:
        print('end herbivores mean',round(statistics.mean(r[2] for r in rows),2),'fear change mean',round(statistics.mean(r[5]-r[4] for r in rows),5))
