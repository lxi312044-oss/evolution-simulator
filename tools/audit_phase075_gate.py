#!/usr/bin/env python3
"""Phase 07.5 integration gate; stdlib, no simulation mutations."""
import argparse, csv, json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
REQUIRED={
 'toxins': 'crates/sim-core/src/toxin.rs',
 'plants': 'crates/sim-core/src/plants/mod.rs',
 'herbivores': 'crates/sim-core/src/animals/mod.rs',
 'carnivores': 'crates/sim-core/src/animals/carnivore.rs',
 'design': 'docs/phase07/PHASE0746.md',
 'baseline': 'docs/phase07/phase0731-baseline/standard-toxin-seed12345.resource-reproduction.csv',
 'regression': 'tools/verify_phase0733.ps1',
}
def evaluate(root=ROOT):
    checks={key:(root/rel).is_file() for key,rel in REQUIRED.items()}
    checks['independent_toxin_types']='pub struct ToxinTypeId' in (root/REQUIRED['toxins']).read_text(encoding='utf-8') if checks['toxins'] else False
    checks['plant_toxic_trait']='produces_toxin: bool' in (root/REQUIRED['plants']).read_text(encoding='utf-8') if checks['plants'] else False
    checks['animal_design_boundary']='Phase 08' in (root/REQUIRED['design']).read_text(encoding='utf-8') if checks['design'] else False
    report_path=root/'results/phase0745/REPORT.json'
    checks['phase0745_report_present']=report_path.is_file()
    if report_path.is_file():
        try:
            report=json.loads(report_path.read_text(encoding='utf-8-sig'))
            checks['phase0745_report_pass']=report.get('status')=='PASS'
        except (ValueError,UnicodeError): checks['phase0745_report_pass']=False
    else: checks['phase0745_report_pass']=False
    csv_files=sorted((root/'results/phase0745').glob('*-seed*.csv'))
    checks['phase0745_nine_csv']=len(csv_files)==9 and all(f.stat().st_size>100 for f in csv_files)
    pending={
        'official_07_5_multi_toxin':'NOT_IMPLEMENTED',
        'official_07_6_color_learning':'NOT_IMPLEMENTED',
        'official_07_7_plant_mimicry':'NOT_VERIFIED',
        'phase0746_design_approval':'PENDING_USER_APPROVAL',
        'animal_toxin_production_transfer':'DEFERRED_TO_PHASE08',
    }
    return {'phase':'07.5-integration-gate','scope':'static evidence check; Rust/regression verified separately',
            'static_status':'PASS' if all(checks.values()) else 'FAIL', 'checks':checks,
            'phase08_decision':'NO-GO','reason':'Formal 07.5/07.6/07.7 acceptance criteria and user approval are pending',
            'pending':pending,'rust_tests':'NOT_RUN_BY_PYTHON','frozen_regression':'NOT_RUN_BY_PYTHON',
            'web_build':'NOT_RUN_BY_DEFAULT','long_run':'NOT_RUN_BY_DEFAULT'}
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--output',default='results/phase075-gate/REPORT.json');a=p.parse_args()
    report=evaluate();dest=ROOT/a.output;dest.parent.mkdir(parents=True,exist_ok=True)
    dest.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
    print(f"{report['static_status']} Phase 07.5 static integration evidence; Phase 08: {report['phase08_decision']}")
    print('Report:',dest.relative_to(ROOT))
    raise SystemExit(0 if report['static_status']=='PASS' else 1)
