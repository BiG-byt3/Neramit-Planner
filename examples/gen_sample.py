"""Generate examples/sample.neramit - a small construction schedule used for demos and screenshots."""
import json
import sys
from pathlib import Path

tasks = [
    # (name, level, duration, preds[(id, type, lag)], resources, percent, milestone)
    ("โครงการก่อสร้างอาคารสำนักงาน 3 ชั้น", 1, 0, [], [], 0),
    ("งานเตรียมการ", 2, 0, [], [], 0),
    ("ขออนุญาตก่อสร้าง", 3, 5, [], ["วิศวกรโครงการ"], 100),
    ("สำรวจและวางผัง", 3, 3, [(3, "FS", 0)], ["ช่างสำรวจ"], 100),
    ("ติดตั้งสำนักงานสนาม", 3, 2, [(4, "SS", 1)], ["คนงาน"], 60),
    ("งานฐานราก", 2, 0, [], [], 0),
    ("ตอกเสาเข็ม", 3, 8, [(4, "FS", 0)], ["ผู้รับเหมาเข็ม"], 30),
    ("ขุดดินและเทฐานราก", 3, 6, [(7, "FS", 0)], ["คนงาน", "คอนกรีต"], 0),
    ("ตรวจรับงานฐานราก", 3, 0, [(8, "FS", 0)], ["วิศวกรโครงการ"], 0),
    ("งานโครงสร้าง", 2, 0, [], [], 0),
    ("เสาและคานชั้น 1", 3, 7, [(9, "FS", 0)], ["คนงาน", "เหล็กเสริม"], 0),
    ("พื้นชั้น 2", 3, 5, [(11, "FS", 0)], ["คนงาน", "คอนกรีต"], 0),
    ("เสาและคานชั้น 2-3", 3, 10, [(12, "FS", 2)], ["คนงาน", "เหล็กเสริม"], 0),
    ("หลังคา", 3, 6, [(13, "FS", 0)], ["ช่างหลังคา"], 0),
    ("งานสถาปัตย์และระบบ", 2, 0, [], [], 0),
    ("ก่อผนังและฉาบปูน", 3, 12, [(13, "SS", 5)], ["ช่างปูน"], 0),
    ("ระบบไฟฟ้าและประปา", 3, 10, [(16, "SS", 3)], ["ช่างไฟฟ้า"], 0),
    ("ทาสีและตกแต่ง", 3, 8, [(16, "FS", 0), (14, "FS", 0)], ["ช่างสี"], 0),
    ("ส่งมอบงาน", 2, 0, [(17, "FS", 0), (18, "FS", 0)], ["วิศวกรโครงการ"], 0),
]

resources = [
    ("วิศวกรโครงการ", "Work", 100, 450, 0),
    ("ช่างสำรวจ", "Work", 100, 300, 0),
    ("คนงาน", "Work", 800, 50, 0),
    ("ผู้รับเหมาเข็ม", "Work", 100, 600, 5000),
    ("คอนกรีต", "Material", 100, 2400, 0),
    ("เหล็กเสริม", "Material", 100, 18000, 0),
    ("ช่างหลังคา", "Work", 300, 80, 0),
    ("ช่างปูน", "Work", 400, 75, 0),
    ("ช่างไฟฟ้า", "Work", 200, 90, 0),
    ("ช่างสี", "Work", 200, 70, 0),
]

start = sys.argv[1] if len(sys.argv) > 1 else "2026-10-19"
res_uid = {name: i + 1 for i, (name, *_rest) in enumerate(resources)}
out_tasks = []
for i, (name, level, dur, preds, res, pct) in enumerate(tasks):
    is_ms = dur == 0 and name in ("ตรวจรับงานฐานราก", "ส่งมอบงาน")
    out_tasks.append(
        {
            "uid": i + 1,
            "name": name,
            "outline_level": level,
            "duration": dur if dur else (0 if is_ms else 1),
            "manual": False,
            "start": start,
            "finish": start,
            "percent_complete": pct,
            "predecessors": [{"pred_uid": p, "kind": k, "lag": lag} for p, k, lag in preds],
            "resources": [res_uid[r] for r in res],
            "notes": "ต้องได้รับใบอนุญาตก่อนเริ่มงานฐานราก" if i == 2 else "",
            "constraint_date": None,
            "milestone": is_ms,
            "on_timeline": level == 2 or is_ms,
            "active": True,
            "collapsed": False,
            "bold": False,
            "italic": False,
            "underline": False,
        }
    )

out_res = []
for i, (name, rtype, max_units, rate, cpu) in enumerate(resources):
    out_res.append(
        {
            "uid": i + 1,
            "name": name,
            "rtype": rtype,
            "material_label": "ตัน" if name == "เหล็กเสริม" else ("ลบ.ม." if name == "คอนกรีต" else ""),
            "initials": name[0],
            "group": "ทีมงาน" if rtype == "Work" else "วัสดุ",
            "max_units": float(max_units),
            "std_rate": float(rate),
            "ovt_rate": float(rate) * 1.5 if rtype == "Work" else 0.0,
            "cost_per_use": float(cpu),
        }
    )

doc = {
    "format": "Neramit Planner",
    "version": 1,
    "project": {
        "title": "อาคารสำนักงาน 3 ชั้น",
        "author": "",
        "company": "",
        "manager": "",
        "start": start,
        "status_date": None,
        "calendar": {"name": "Standard", "working_days": [True] * 5 + [False] * 2, "hours_per_day": 8.0, "holidays": []},
        "tasks": out_tasks,
        "resources": out_res,
        "next_uid": len(out_tasks) + 1,
        "next_res_uid": len(out_res) + 1,
        "new_tasks_auto": True,
    },
}
path = Path(__file__).with_name("sample.neramit")
path.write_text(json.dumps(doc, ensure_ascii=False, indent=1), encoding="utf-8")
print("wrote", path)
