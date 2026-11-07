# Neramit Planner 1.0

โปรแกรมวางแผนงานโครงการแบบออฟไลน์พร้อม Gantt Chart เขียนด้วยภาษา Rust
เปิด แก้ไข และบันทึกไฟล์ .mpp / .mpt และ XML ได้โดยตรง ไม่ต้อง Login ใช้แบบ exe ไฟล์เดียว หรือติดตั้งด้วย `NeramitPlanner-Setup.exe` (ไม่ต้องใช้สิทธิ์ Admin)
ภาษาอังกฤษเป็นค่าเริ่มต้น (สลับเป็นภาษาไทยได้ที่ปุ่มภาษาบนแถบชื่อ หรือ File > Options > Language โปรแกรมจะจำภาษาที่เลือกไว้)

**Neramit Planner Made by BiGByte Studio**

## ดาวน์โหลด

ดาวน์โหลดโปรแกรมสำหรับ Windows 64 บิตจาก [GitHub Releases เวอร์ชันล่าสุด](https://github.com/BiG-byt3/Neramit-Planner/releases/latest)

- **ติดตั้ง:** เลือกไฟล์ `NeramitPlanner-Setup-<version>.exe` ติดตั้งสำหรับผู้ใช้ปัจจุบันโดยไม่ต้องใช้สิทธิ์ Admin
- **ใช้ไฟล์เดียว (Portable):** เลือกไฟล์ `NeramitPlanner-<version>.exe` แล้วเปิดใช้งานได้ทันที
- ตรวจอัปเดตจากในโปรแกรมที่ **Help > Check for Updates** และเลือก **Update now** เมื่อมีเวอร์ชันใหม่

ไฟล์ `.exe` และ manifest สำหรับอัปเดตอยู่ใน Releases ส่วน repository นี้เก็บซอร์สโค้ดและเครื่องมือสร้างโปรแกรม

## ความสามารถ

| ส่วน | รายละเอียด |
|---|---|
| หน้าจอ | Ribbon (งาน, ทรัพยากร, รายงาน, โครงการ, มุมมอง, รูปแบบ, วิธีใช้), File backstage, Timeline, Details (Task Form), Status bar พร้อม Zoom |
| มุมมอง | Gantt Chart, Tracking Gantt, Task Sheet, Task Usage, Network Diagram, Calendar, Task Board, Sprint Planning Board, Resource Sheet, Resource Usage, Reports |
| ตาราง | Entry, Cost, Hyperlink, Schedule, Summary, Tracking, Usage, Variance, Work + เพิ่ม/ซ่อนคอลัมน์ทุกฟิลด์, Custom Fields, จัดกลุ่ม/กรอง/เน้น/เรียงลำดับ, Project Summary Task |
| การคำนวณ | Auto/Manual (Start/Finish/Duration เป็นข้อความได้ เช่น "TBD" หรือเว้นว่าง), ลิงก์ 4 ชนิดพร้อม Lag (รวม elapsed lag), Constraint ทุกแบบ, Deadline, ปฏิทินวันทำงาน/ชั่วโมงทำงาน/วันหยุด, Critical Path, Slack, งานแบ่งช่วง (Split), Task type + Effort driven, Leveling (พร้อม Leveling Options), Reschedule ไปยัง Status Date |
| ติดตามงาน | Baseline 0–10, Actual Start/Finish, % Complete, Update Tasks/Project, Progress Lines, แถบ Baseline/Slippage, Task Path |
| ทรัพยากร | Work / Material / Cost, อัตราค่าจ้าง, Cost/Use, Notes, Email, Assign Resources, Next Overallocation, Resource Pool แบบเชื่อมต่อจริง (Share Resources, Refresh / Update Resource Pool, รีเฟรชจาก pool อัตโนมัติเมื่อเปิดไฟล์) |
| ฟิลด์ | Custom Fields (Text/Number/Flag/Date/Cost/Duration/Start/Finish) พร้อมชื่อแทน ตาราง Lookup และสูตร, WBS (Define Code + Renumber) |
| รายงาน | Dashboards (Project/Cost/Work Overview, Burndown, Upcoming Tasks), Resources, Costs (Cash Flow, Cost Overruns, Earned Value), In Progress (Critical/Late/Slipping Tasks, Milestones, S-Curve), Task Status, Custom, Compare Projects |
| อื่น ๆ | Undo/Redo, Format Painter, Drawing บน Gantt, Insert Subproject, Manage Sprints, Macros (บันทึกและเล่นซ้ำคำสั่ง เก็บถาวรใน macros.json), Spelling (ตัวตรวจคำสะกดของ Windows), New Window / Switch Windows / Arrange All / Hide / Unhide (หลายหน้าต่างในโปรแกรมเดียว), Links Between Projects (ลิงก์ข้ามโครงการ งานภายนอก), พิมพ์/Export PDF, Excel, CSV |

## สำหรับผู้ใช้ไทย

- **วันหยุดไทย** (โครงการ > เปลี่ยนเวลาทำงาน > เพิ่มวันหยุดไทย): เพิ่มวันหยุดนักขัตฤกษ์และวันหยุดชดเชยให้ทุกปีตลอดช่วงโครงการ
  วันหยุดที่วันที่ตายตัวคำนวณได้ทุกปี วันชดเชยใช้กฎเดียวกับมติ ครม. (วันหยุดตรงเสาร์-อาทิตย์ ชดเชยวันทำงานถัดไปที่ไม่ใช่วันหยุด)
  วันหยุดทางพุทธศาสนา วันพืชมงคล และวันหยุดพิเศษ มีในตัวโปรแกรมสำหรับปี 2568–2570 (ปี 2570 เป็นวันที่ประมาณการจนกว่า ครม. จะประกาศ)
  เอาติ๊ก "รวมวันหยุดเฉพาะราชการ" ออกได้ (วันพืชมงคล วันเข้าพรรษา)
- ปีอื่นเพิ่มเองได้โดยไม่ต้องรออัปเดต: สร้างไฟล์ `thai-holidays.txt` ในโฟลเดอร์ค่าตั้ง (`%APPDATA%\Neramit Planner`) บรรทัดละหนึ่งวัน
  ใช้ปี พ.ศ. หรือ ค.ศ. ก็ได้ ใส่ `*` ท้ายบรรทัดสำหรับวันหยุดเฉพาะราชการ:

  ```
  # วันหยุดทางพุทธศาสนา ปี 2571
  2571-02-10 วันมาฆบูชา
  2571-07-07 วันเข้าพรรษา *
  ```
- **แบบสำเร็จรูปวันทำงาน**: สำนักงาน (จ.–ศ.) และไซต์ก่อสร้าง (จ.–ส.)
- **รายงาน S-Curve** (รายงาน > กำลังดำเนินการ > กราฟ S-Curve): ผลงานตามแผนสะสมเทียบผลงานจริงสะสม ถ่วงน้ำหนักตามค่างาน
  (หรือชั่วโมงงาน / ระยะเวลา) เส้นแผนใช้ Baseline ถ้ามี และแสดงเส้นตามกำหนดการปัจจุบันเป็นเส้นคาดการณ์
  มีตารางรายสัปดาห์/รายเดือน ตารางน้ำหนักงานแยกตามหมวดงาน และปุ่มคัดลอกตารางไปวางใน Excel
- **สกุลเงิน**: โครงการใหม่ของผู้ใช้ไทยเป็น ฿ / THB และเงินทุกช่องแสดงตามตัวเลือกสกุลเงินของโครงการ
- **เรียงตามชื่อ** ใช้ลำดับพจนานุกรมไทย (เ แ โ ใ ไ เรียงตามพยัญชนะต้น)
- **ส่งออก Excel** ที่มีภาษาไทยใช้ฟอนต์ Leelawadee UI
- **คัดลอกเป็นรูปภาพ / บันทึกเป็นรูปภาพ** (มุมมอง > แชร์ หรือ รายงาน > แชร์): มุมมองที่เห็นบนจอ (Gantt ตาราง รายงาน) เป็นรูปภาพ
  วางใน LINE อีเมล หรือเอกสารได้ทันที หรือบันทึกเป็นไฟล์ PNG
- **งานพิมพ์ / PDF**: หัวกระดาษมีชื่อบริษัท ผู้จัดการโครงการ (จากข้อมูลโครงการ) และ "ข้อมูล ณ วันที่" (วันที่สถานะ)
  วันที่พิมพ์เขียนแบบเต็ม เช่น 5 พฤศจิกายน 2569 และมีช่องลงนาม ผู้จัดทำ / ผู้ตรวจสอบ / ผู้อนุมัติ ท้ายหน้าสุดท้าย
  (เปิด/ปิดได้ที่ ไฟล์ > พิมพ์ ค่าเริ่มต้นเปิดสำหรับผู้ใช้ไทย)
- **จำนวนเงินเป็นตัวหนังสือ**: รายงานภาพรวมโครงการและรายงานต้นทุน แสดงยอดต้นทุนเป็นคำอ่าน เช่น (หนึ่งร้อยเอ็ดบาทถ้วน)
- **นำเข้า BOQ** (โครงการ > นำเข้า BOQ): ไฟล์ Excel (.xlsx) หรือ CSV ของบัญชีแสดงปริมาณงานและราคา (แบบ ปร.4 หรือแบบของบริษัทเอง)
  กลายเป็นโครงร่างงาน: หมวดเป็นงานสรุป รายการเป็นงานที่มีจำนวนเงินเป็นต้นทุนคงที่ (ใช้ถ่วงน้ำหนัก S-Curve และงวดงานได้ทันที)
  - หาหัวตารางเองจากคำ ลำดับ / รายการ / จำนวน / หน่วย / ค่าวัสดุ / ค่าแรง / รวม รวมหัวตาราง 2 แถวที่ผสานเซลล์ (ราคาต่อหน่วย | จำนวนเงิน)
  - เลือกชีตที่มีรายการมากที่สุดให้เอง (ข้ามชีตสรุป ปร.5) และแก้แถวหัวตารางหรือคอลัมน์ได้ในหน้าต่าง พร้อมดูตัวอย่างและยอดรวมก่อนนำเข้า
  - ลำดับชั้นจากเลขข้อ (1, 1.1, 1.1.1 / ก, ข) ถ้าไม่มีเลข แถวที่ไม่มีจำนวนและเงินถือเป็นหมวด
  - ข้ามแถวรวมหมวด ยกยอด ภาษีมูลค่าเพิ่ม Factor F และรวมทั้งสิ้น บรรทัดคำอธิบาย ("- รวมค่าปั๊ม ...") ไปอยู่ใน Notes ของงาน
  - ไม่มีคอลัมน์รวม ใช้ ค่าวัสดุ + ค่าแรง หรือ จำนวน × ราคาต่อหน่วย; ตั้งระยะเวลาต่อรายการและเชื่อมงานต่อกันตามลำดับได้
  - อ่าน .xls รุ่นเก่าไม่ได้ (บันทึกเป็น .xlsx ก่อน) CSV อ่านได้ทั้ง UTF-8 และ Windows-874
- **งวดงาน/งวดเงิน** (โครงการ > งวดงาน/งวดเงิน และ รายงาน > ต้นทุน > งวดงาน/งวดเงิน):
  - มูลค่าสัญญา (0 = ใช้ต้นทุนรวมของโครงการ) เงินล่วงหน้า % (หักคืนจากทุกงวดตามสัดส่วน) และเงินประกันผลงาน % (หักไว้ทุกงวด)
  - แต่ละงวดเป็น % ของสัญญาหรือจำนวนเงินคงที่ ผูกกับงาน (ปกติคือเหตุการณ์สำคัญ) ที่เสร็จแล้วจึงเบิกได้ และติ๊กว่ารับเงินแล้ว
  - "สร้างจากเหตุการณ์สำคัญ" ทำหนึ่งงวดต่อหนึ่งเหตุการณ์สำคัญ คิดสัดส่วนจากมูลค่างานที่เสร็จนับจากเหตุการณ์สำคัญก่อนหน้า (รวม 100%)
  - รายงานแสดงสถานะ (รับเงินแล้ว / ถึงกำหนดเบิก / งานล่าช้า / ยังไม่ถึงกำหนด) กำหนดเทียบแผนเดิม (Baseline) ยอดหัก รับสุทธิ
    มูลค่าสัญญาเป็นตัวหนังสือ และกราฟเงินรับสะสมเทียบต้นทุนสะสมรายเดือน
  - บันทึกในไฟล์ .neramit .mpp และ .xml (โปรแกรมอื่นข้ามข้อมูลนี้ไป เปิดไฟล์ได้ปกติ)

## รูปแบบไฟล์

| นามสกุล | อ่าน | เขียน | หมายเหตุ |
|---|:-:|:-:|---|
| `.neramit` | ✓ | ✓ | ไฟล์ของ Neramit Planner (JSON) |
| `.mpp` / `.mpt` | ✓ | ✓ | รูปแบบ MPP12 และ MPP14 (ไม่เขียนทับไฟล์ต้นฉบับ: Save บันทึกเป็นไฟล์ใหม่) ไฟล์ MPP12 บันทึกเป็นรูปแบบ MPP14 |
| `.xml` | ✓ | ✓ | Project XML |
| `.pdf` `.xlsx` `.csv` | | ✓ | Export |

การบันทึก .mpp สร้างตารางข้อมูลของไฟล์ใหม่ทั้งหมด (`src/mppt.rs`) รองรับการเพิ่ม/ลบ/ย้ายงาน ลิงก์ ทรัพยากร
การมอบหมายงาน ปฏิทิน Notes Custom Fields (ค่า ชื่อแทน ตาราง Lookup และสูตร) WBS Sprints และ Task Board
Resource Pool และลิงก์ข้ามโครงการ (งานภายนอก) ส่วนโครงการที่สร้างใหม่ใน Neramit Planner บันทึกเป็น .mpp ได้เช่นกัน
Drawing บน Gantt เก็บในรายการข้อมูลของ Neramit Planner เองใน .mpp และใน comment ของ .xml (โปรแกรมอื่นข้ามไป เปิดไฟล์ได้ปกติ)

แปลงไฟล์ผ่าน command line:

```
NeramitPlanner.exe --convert input.mpp output.xml
NeramitPlanner.exe --convert project.xml project.mpp
```

## ติดตั้ง

`NeramitPlanner-Setup.exe` เป็นตัวติดตั้งแบบ Wizard (ภาษาไทยเมื่อ Windows เป็นภาษาไทย นอกนั้นภาษาอังกฤษ)
ติดตั้งให้ผู้ใช้ Windows คนปัจจุบัน ไม่ต้องใช้สิทธิ์ Admin:

1. Welcome
2. ข้อตกลงการใช้งาน (`installer/license-en.txt` / `license-th.txt`) ต้องเลือกยอมรับก่อนจึงกด Next ได้
3. เลือกโฟลเดอร์ (ค่าเริ่มต้น `%LOCALAPPDATA%\Programs\Neramit Planner`)
4. เลือกงานเพิ่มเติม (ติ๊กไว้ทั้งหมดเป็นค่าเริ่มต้น): ทางลัดบน Desktop / Start menu, ให้ `.mpp` `.mpt` เปิดด้วย Neramit Planner เป็นค่าเริ่มต้น, เมนูคลิกขวา "Open with Neramit Planner" (.mpp .mpt .xml),
   เชื่อมต่อกับผู้ช่วย AI ในเครื่อง (ติ๊กไว้เมื่อพบ Claude Desktop, Claude Code หรือ Codex ดูหัวข้อ "เชื่อมต่อผู้ช่วย AI (MCP)")
5. สรุป แล้วกด Install (ถ้าโปรแกรมเปิดอยู่ จะขอให้ปิดก่อน)
6. Finish (เลือกเปิดโปรแกรมได้)

- ไฟล์ `.neramit` ดับเบิลคลิกเปิดได้เสมอ และ Neramit Planner แสดงใน Settings > Default apps
- ถ้าผู้ใช้เคยเลือก "Always" ให้โปรแกรมอื่นเปิด `.mpp` ไว้ Windows ไม่ยอมให้โปรแกรมใดเปลี่ยนค่านั้นเอง
  หน้า Finish จะมีปุ่มเปิดหน้าต่างเลือกโปรแกรมของ Windows ให้ (ทำซ้ำได้ที่ File > Options > General > File types)
- ติดตั้งซ้ำเมื่อมีเวอร์ชันเก่าอยู่แล้วจะอัปเดตในโฟลเดอร์เดิม ด้วยตัวเลือกเดิม
- ถอนการติดตั้งได้ที่ Settings > Apps > Installed apps: คืนค่าโปรแกรมเดิมของ `.mpp` ไฟล์งานและค่าตั้งใน `%APPDATA%\Neramit Planner` ยังอยู่
- `NeramitPlanner-Setup.exe /S` ติดตั้งแบบไม่ถาม (ใช้ตอนอัปเดตออนไลน์ ทับได้แม้โปรแกรมยังเปิดอยู่)

## เชื่อมต่อผู้ช่วย AI (MCP)

`NeramitPlanner.exe --mcp` ทำงานเป็น MCP server (stdio) ให้ผู้ช่วย AI เช่น Claude Code, Claude Desktop และ Codex
เปิด อ่าน แก้ไข และบันทึกแผนงานได้ โดยใช้โค้ดชุดเดียวกับคำสั่งในโปรแกรม (แปลงค่าเซลล์ ตรวจลิงก์วน คำนวณตาราง Undo)

- การแก้ไขอยู่ในหน่วยความจำจนกว่าจะเรียก `save` และไม่เขียนทับ .mpp ต้นฉบับ
  ไม่บันทึกทับไฟล์ที่ถูกแก้บนดิสก์หลังเปิด (เช่น บันทึกจากโปรแกรม) เว้นแต่ส่ง `overwrite: true`
- ทุกการแก้ไขตอบกลับว่างานไหนเลื่อน และวันจบโครงการเปลี่ยนกี่วันทำงาน ส่วน `dry_run: true` ถาม "ถ้า...จะเป็นอย่างไร" โดยไม่แก้จริง
- ไม่แตะค่าตั้ง ไฟล์ล่าสุด และรายการโปรดของผู้ใช้

| กลุ่ม | เครื่องมือ |
|---|---|
| เปิด / ปิด | `open_project` `new_project` `list_open_projects` `close_project` |
| อ่าน | `project_summary` `list_tasks` (กรอง: critical, late, slipping, milestones ... / ช่วงวันที่ / ค้นชื่อ / ทรัพยากร) `get_task` `critical_path` `list_resources` `progress_report` `compare_projects` |
| แก้ไข | `add_tasks` (ทั้งโครงร่างในครั้งเดียว อ้างงานในชุดเดียวกันด้วย `@1`, `@2SS+1d`) `update_tasks` `delete_tasks` `link_tasks` `unlink_tasks` `add_resources` `update_project` (ข้อมูลโครงการ วันทำงาน วันหยุด) `set_baseline` `level_resources` `undo` |
| BOQ / งวดงาน | `import_boq` (BOQ .xlsx/.csv เป็นงาน) `payment_schedule` `set_payment_schedule` (เงื่อนไขสัญญา งวด หรือสร้างจากเหตุการณ์สำคัญ) |
| บันทึก | `save` (.neramit .xml หรือ .mpp ไฟล์ใหม่) `export` (.pdf .xlsx .csv .xml) |

เปิด/ปิดการเชื่อมต่อทีละตัวได้ที่ Help > AI Assistants (หรือ File > Options > AI Assistants) ซึ่งแสดงสถานะของ Claude Code, Claude Desktop และ Codex
ลงทะเบียนกับผู้ช่วย AI: ติ๊กใน Setup หรือสั่ง `NeramitPlanner.exe --connect-ai` (ถอนด้วย `--disconnect-ai`; ถอนการติดตั้งก็ถอนให้)
ซึ่งเพิ่มเฉพาะรายการ `neramit-planner` ใน `%APPDATA%\Claude\claude_desktop_config.json`, `%USERPROFILE%\.codex\config.toml`
และ `claude mcp add --scope user` ของ Claude Code หรือเพิ่มเองได้:

```
claude mcp add --scope user neramit-planner -- "%LOCALAPPDATA%\Programs\Neramit Planner\NeramitPlanner.exe" --mcp
```

## อัปเดตออนไลน์

กด **Help > Check for Updates** (หรือปุ่ม Check for Updates ในหน้า About) เพื่อตรวจเวอร์ชันใหม่จาก
[GitHub Releases](https://github.com/BiG-byt3/Neramit-Planner/releases/latest) แล้วเลือก **Update now** เพื่อดาวน์โหลดจากในโปรแกรม
โปรแกรมตรวจลายเซ็น ECDSA P-256 ของ BiGByte Studio และ SHA-256 ก่อนใช้ชุดอัปเดตทุกครั้ง

- **รุ่นติดตั้ง:** ดาวน์โหลด Setup แล้วติดตั้งทับแบบเงียบและเปิดโปรแกรมอีกครั้ง
- **รุ่น Portable:** ดาวน์โหลดไฟล์โปรแกรมใหม่แล้วแทนที่ `.exe` เดิมและเปิดโปรแกรมอีกครั้ง โดยโฟลเดอร์นั้นต้องเขียนไฟล์ได้
- หากมีงานที่ยังไม่บันทึก โปรแกรมจะให้บันทึกก่อนปิดเพื่ออัปเดต
- การตรวจอัปเดตต้องต่ออินเทอร์เน็ต งานวางแผนโครงการยังใช้งานออฟไลน์ได้ตามปกติ

ออกเวอร์ชันใหม่:

1. เพิ่ม `version` ใน `Cargo.toml` และ `installer/Cargo.toml` ให้ตรงกัน (เช่น `1.0.1`) แล้ว build และทดสอบโปรแกรมใหม่
2. build installer หลัง build โปรแกรมเสร็จ: `cargo build --release --locked --manifest-path installer/Cargo.toml`
3. เขียน `dist/notes-<version>.txt` (ภาษาอังกฤษ / บรรทัด `---` / ภาษาไทย)
4. ติดตั้ง Python dependency ครั้งแรก: `python -m pip install cryptography`
5. `python tools/release.py sign` ใช้กุญแจลับเดิมใน `%USERPROFILE%\.neramit-release\update-signing-key.pem` (หรือระบุไฟล์ด้วย `NERAMIT_RELEASE_KEY`)
6. `python tools/release.py verify` ตรวจไฟล์ใน `dist/<version>/` ว่าตรงกับลายเซ็นและ checksum
7. นำซอร์สโค้ดขึ้น `main` แล้วตั้ง `NERAMIT_RELEASE_TARGET` เป็น commit SHA ที่ build มา
8. ตั้ง `GITHUB_TOKEN` ที่มีสิทธิ์ Contents: read and write สำหรับ repository นี้ แล้วรัน `python tools/release.py publish`

เครื่องมือเผยแพร่สร้าง Release แบบ Draft และอัปโหลดไฟล์ครบก่อนเปิดให้ดาวน์โหลด:
`NeramitPlanner-Setup-<version>.exe`, `NeramitPlanner-<version>.exe`, `latest.json`, `SHA256SUMS.txt` และ `release-notes.txt`
หากอัปโหลดขาดช่วง รัน `publish` ซ้ำเพื่อทำ Draft เดิมต่อได้ เครื่องมือจะไม่เขียนทับเวอร์ชันที่เผยแพร่แล้ว
หรืออัปโหลดไฟล์ชุดเดียวกันผ่านหน้า GitHub Releases ด้วยตนเอง โดยใช้แท็ก `v<version>` และกำหนดเป็น Latest release

ระบบอัปเดตอ่าน `latest.json` จาก Release ล่าสุด ดังนั้นต้องแนบไฟล์นี้และไฟล์ `.exe` ชื่อดังกล่าวทุกครั้ง
การ push ซอร์สโค้ดเพียงอย่างเดียวยังไม่ออกเวอร์ชันใหม่ และไม่มี workflow ที่เผยแพร่หรือใช้กุญแจลับโดยอัตโนมัติ

กุญแจลับต้องสำรองไว้และห้ามอัปโหลด เครื่องมือจะปฏิเสธกุญแจที่ไม่ตรงกับ public key ในโปรแกรม
ถ้ากุญแจหาย โปรแกรมที่แจกไปแล้วจะรับอัปเดตที่ลงลายเซ็นด้วยกุญแจใหม่ไม่ได้

หลังเผยแพร่ ตรวจ manifest และดาวน์โหลดไฟล์ทั้งสองแบบเพื่อตรวจลายเซ็นและ checksum โดยไม่ติดตั้ง:

```powershell
& .\target\release\NeramitPlanner.exe --check-update | Out-Host
& .\target\release\NeramitPlanner.exe --verify-update-download | Out-Host
```

คำสั่งคืน exit code `0` เมื่อผ่าน และ `1` เมื่อเชื่อมต่อหรือยืนยันชุดอัปเดตไม่สำเร็จ

## Build

ใช้ Rust toolchain `x86_64-pc-windows-gnu` ใน `E:\REA\.toolchains`:

```powershell
$env:CARGO_HOME='E:\REA\.toolchains\rust\cargo'
$env:RUSTUP_HOME='E:\REA\.toolchains\rust\rustup'
$env:Path="E:\REA\.toolchains\rust\cargo\bin;E:\REA\.toolchains\binutils\mingw64\bin;$env:Path"
$env:WINDRES='E:\REA\.toolchains\binutils\windres-wrapper.exe'
$env:AR='E:\REA\.toolchains\binutils\mingw64\bin\ar.exe'
cargo build --release --offline
cargo test --release --offline
```

ไฟล์โปรแกรมอยู่ที่ `target\release\NeramitPlanner.exe` (ไอคอนและข้อมูลเวอร์ชันฝังผ่าน `build.rs`
ซึ่งคอมไพล์ resource ในโฟลเดอร์ชั่วคราว เพราะ windres ใช้กับโฟลเดอร์ที่มีช่องว่างในชื่อไม่ได้)
โลโก้ที่เลือกใช้: `tools/logo/beaver-poses-20261107/02-over-shoulder-1024.png` (บีเวอร์หันมองข้ามไหล่ ลายเส้นแกะสลักขาวดำบนพื้นเขียว `#2B8238` ของโปรแกรม) ภาพที่ใช้งานอยู่ที่ `tools/logo/neramit-planner-logo-1024.png` และ `tools/logo/neramit-planner-beaver-1024.png` สร้างไฟล์ `.ico` และ `.rgba` สำหรับโปรแกรมและตัวติดตั้งใหม่ได้ด้วย `python tools/logo/export_beaver_over_shoulder.py` (ใช้ Pillow และคัดลอกภาพที่เลือกตรงตามต้นฉบับ) ส่วนไอคอนก่อนหน้าเก็บไว้ที่ `tools/logo/archive/pre-beaver-over-shoulder-20261107/` คำสั่งสร้างภาพอยู่ที่ `tools/logo/beaver-poses-20261107/02-over-shoulder-prompt.txt`

ทดสอบกับไฟล์จริง (ตั้งตัวแปรแล้วรัน `cargo test`):

| ตัวแปร | ใช้ทดสอบ |
|---|---|
| `NERAMIT_MPP_TESTDATA` | โฟลเดอร์ไฟล์คู่ name.mpp + name.xml: ตัวอ่าน .mpp ตรงกับ XML ของไฟล์เดียวกัน |
| `NERAMIT_MPPT_EDIT` | `input.mpp;โฟลเดอร์ผลลัพธ์;scenario,...` เขียนสำเนาที่แก้ไขแล้วเพื่อตรวจ |
| `NERAMIT_MPPT_NEW` | โฟลเดอร์ผลลัพธ์: โครงการใหม่ทั้งไฟล์ |
| `NERAMIT_CALDIR` / `NERAMIT_CFDIR` | ไฟล์ตัวอย่างปฏิทิน / Custom Fields ทีละค่า |
| `NERAMIT_MPPT_CONVERT` | `input.mpp;โฟลเดอร์ผลลัพธ์`: ทั้งไฟล์เป็นแถวใหม่ของ template (เส้นทางของไฟล์ MPP12) |
| `NERAMIT_LINK_SAMPLES` | โฟลเดอร์ที่มี sharer.mpp และ main2.mpp (Resource Pool และลิงก์ข้ามโครงการ) |
| `NERAMIT_SPRINT_MPP` / `NERAMIT_POOL_DIR` | ไฟล์ตัวอย่าง Sprints / โฟลเดอร์สำหรับทดสอบ Resource Pool |

ทดสอบหน้าจอแบบไม่เปิดหน้าต่าง: `NeramitPlanner.exe --qa-script steps.txt` (ดูคำสั่งใน `src/qa.rs`; `bench N` วัดเวลาต่อเฟรม, `mem` วัดหน่วยความจำ)

## โครงสร้างโค้ด

| ไฟล์ | หน้าที่ |
|---|---|
| `src/model.rs` | ข้อมูล Project / Task / Resource / Calendar |
| `src/undo.rs` | Undo / Redo (โครงการใหญ่บีบอัดขั้นเก่าไว้เบื้องหลัง) |
| `src/mcp.rs` `src/aiconnect.rs` | MCP server สำหรับผู้ช่วย AI / ลงทะเบียนกับ Claude และ Codex |
| `src/schedule.rs` | Engine คำนวณ, Leveling |
| `src/mpp.rs` | อ่าน .mpp |
| `src/mppt.rs` | บันทึก .mpp |
| `src/mppw.rs` | เขียนไฟล์ OLE compound |
| `src/fileio.rs` | .neramit, Project XML, CSV |
| `src/custom.rs` `src/custom_tables.rs` | Custom Fields และสูตร |
| `src/cfdefs.rs` `src/field_ids.rs` | ชื่อแทน สูตร และตาราง Lookup ใน .mpp / รายชื่อฟิลด์ตามหมายเลข |
| `src/sprintio.rs` | Sprints และ Task Board ใน .mpp |
| `src/extlinks.rs` | ลิงก์ไปไฟล์อื่น: Resource Pool และลิงก์ข้ามโครงการ (OLE file moniker) |
| `src/wbs.rs` | WBS code mask |
| `src/reports.rs` `src/compare.rs` | รายงาน, Compare Projects |
| `src/board.rs` | Task Board, Sprint Planning Board |
| `src/drawing.rs` `src/spell.rs` | Drawing, Spelling |
| `src/app.rs` `src/ribbon.rs` `src/grid.rs` `src/gantt.rs` `src/dialogs.rs` `src/taskinfo.rs` | หน้าจอและคำสั่ง |

## คีย์ลัด

Ctrl+N/O/S, F12 (Save As), Ctrl+Z/Y, Insert, Delete, F2, Shift+F2 (Task Information), Ctrl+F2 / Ctrl+Shift+F2 (Link / Unlink),
Alt+Shift+→/← (Indent/Outdent), Ctrl+C/X/V, Ctrl+D (Fill Down), Ctrl+F (Find), F7 (Spelling), F9 (คำนวณใหม่),
Ctrl+ล้อเมาส์ (Zoom timescale), Shift+ล้อเมาส์ (เลื่อนแนวนอน)

---

© 2026 Neramit Planner Made by BiGByte Studio
