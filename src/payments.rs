//! Payment Schedule (งวดงาน / งวดเงิน): the contract's installments, each due when its task
//! (usually a milestone) is complete, with the advance payment recovered and the retention
//! withheld from every installment as Thai construction contracts do.
//!
//! - Project > Payment Schedule edits the terms and the installments; "From milestones" makes one
//!   installment per milestone, sized by the value of the work finished since the one before.
//! - Report > Costs > Payment Schedule lists them with their status and compares the money in
//!   with the cost going out, month by month.

use crate::app::App;
use crate::model::{Payment, PaymentPlan, Project};
use chrono::NaiveDate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// marked paid
    Paid,
    /// the work is complete: the installment can be billed
    Due,
    /// the work should have been complete by the status date
    Late,
    Planned,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Paid => "Paid",
            Status::Due => "Ready to bill",
            Status::Late => "Work late",
            Status::Planned => "Not yet due",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub name: String,
    /// the task's name (the condition of the installment)
    pub condition: String,
    pub task: Option<usize>,
    pub due: Option<NaiveDate>,
    pub baseline_due: Option<NaiveDate>,
    /// % of the contract
    pub pct: f64,
    pub amount: f64,
    pub advance: f64,
    pub retention: f64,
    pub net: f64,
    pub status: Status,
}

#[derive(Clone, Debug, Default)]
pub struct Schedule {
    pub contract: f64,
    /// the advance payment itself (paid at the start)
    pub advance_amount: f64,
    pub rows: Vec<Row>,
    pub pct_sum: f64,
    pub retention_total: f64,
    pub paid_net: f64,
    pub due_net: f64,
    pub net_total: f64,
}

/// The contract value: as entered, else the project's total cost.
pub fn contract_value(p: &Project) -> f64 {
    if p.payments.contract_value > 0.0 {
        p.payments.contract_value
    } else {
        p.tasks.iter().filter(|t| t.outline_level == 1).map(|t| t.cost).sum()
    }
}

pub fn schedule(p: &Project, status_date: NaiveDate) -> Schedule {
    let plan = &p.payments;
    let contract = contract_value(p);
    let mut s = Schedule { contract, advance_amount: contract * plan.advance_pct / 100.0, ..Default::default() };
    for it in &plan.items {
        let amount = if it.amount > 0.0 { it.amount } else { contract * it.percent / 100.0 };
        let pct = if contract > 0.0 { amount / contract * 100.0 } else { it.percent };
        let task = it.task_uid.and_then(|u| p.tasks.iter().position(|t| t.uid == u));
        let t = task.map(|i| &p.tasks[i]);
        let status = if it.paid {
            Status::Paid
        } else {
            match t {
                Some(t) if t.percent_complete >= 100 => Status::Due,
                Some(t) if t.finish < status_date => Status::Late,
                _ => Status::Planned,
            }
        };
        let advance = amount * plan.advance_pct / 100.0;
        let retention = amount * plan.retention_pct / 100.0;
        let net = amount - advance - retention;
        s.pct_sum += pct;
        s.retention_total += retention;
        s.net_total += net;
        match status {
            Status::Paid => s.paid_net += net,
            Status::Due => s.due_net += net,
            _ => {}
        }
        s.rows.push(Row {
            name: it.name.clone(),
            condition: t.map(|t| t.name.clone()).unwrap_or_default(),
            task,
            due: t.map(|t| t.finish),
            baseline_due: t.and_then(|t| t.baseline(0)).and_then(|b| b.finish).map(|d| d.date()),
            pct,
            amount,
            advance,
            retention,
            net,
            status,
        });
    }
    s
}

/// One installment per milestone, each worth the work finished since the milestone before it
/// (by cost; equal shares when nothing has a cost). The percentages add up to 100.
pub fn from_milestones(p: &Project, thai: bool) -> Vec<Payment> {
    let mut ms: Vec<usize> = (0..p.tasks.len()).filter(|&i| p.tasks[i].active && !p.tasks[i].is_summary && p.tasks[i].is_milestone()).collect();
    ms.sort_by_key(|&i| (p.tasks[i].finish, i));
    if ms.is_empty() {
        return Vec::new();
    }
    let leaves: Vec<&crate::model::Task> = p.tasks.iter().filter(|t| t.active && !t.is_summary).collect();
    let total: f64 = leaves.iter().map(|t| t.cost.max(0.0)).sum();
    let mut shares: Vec<f64> = Vec::new();
    let mut prev: Option<NaiveDate> = None;
    for (k, &m) in ms.iter().enumerate() {
        let f = p.tasks[m].finish;
        let last = k + 1 == ms.len();
        let v: f64 = leaves.iter().filter(|t| prev.is_none_or(|pf| t.finish > pf) && (last || t.finish <= f)).map(|t| t.cost.max(0.0)).sum();
        shares.push(v);
        prev = Some(f);
    }
    let n = ms.len();
    let mut pcts: Vec<f64> = if total > 0.0 { shares.iter().map(|v| (v / total * 10_000.0).round() / 100.0).collect() } else { vec![(10_000.0 / n as f64).floor() / 100.0; n] };
    let sum: f64 = pcts[..n - 1].iter().sum();
    pcts[n - 1] = ((100.0 - sum) * 100.0).round() / 100.0;
    ms.iter()
        .zip(pcts)
        .enumerate()
        .map(|(k, (&m, pct))| Payment {
            name: if thai { format!("งวดที่ {}", k + 1) } else { format!("Installment {}", k + 1) },
            percent: pct,
            amount: 0.0,
            task_uid: Some(p.tasks[m].uid),
            paid: false,
            note: String::new(),
        })
        .collect()
}

// ======================================================================== dialog

#[derive(Clone, Debug)]
pub struct PayState {
    pub plan: PaymentPlan,
    pub error: Option<String>,
}

pub fn open(app: &mut App) {
    app.dialog = Some(crate::dialogs::Dialog::Payments(PayState { plan: app.doc.payments.clone(), error: None }));
}

/// The Payment Schedule dialog; `Some(true)` keeps the changes, `Some(false)` cancels.
pub fn dialog(app: &mut App, ctx: &eframe::egui::Context, st: &mut PayState) -> Option<bool> {
    use crate::i18n::{tr, trf};
    use crate::theme;
    use eframe::egui::{self, RichText};
    let mut done = None;
    let thai = crate::i18n::is_thai();
    let cost_total: f64 = app.doc.tasks.iter().filter(|t| t.outline_level == 1).map(|t| t.cost).sum();
    // tasks to choose from: milestones first, then the other tasks
    let mut choices: Vec<(u32, String)> = Vec::new();
    for pass in 0..2 {
        for t in app.doc.tasks.iter().filter(|t| t.active && !t.is_summary && (t.is_milestone() == (pass == 0))) {
            choices.push((t.uid, if pass == 0 { format!("{} ({})", t.name, tr("milestone")) } else { t.name.clone() }));
        }
    }
    let name_of = |u: Option<u32>| u.and_then(|u| choices.iter().find(|c| c.0 == u)).map(|c| c.1.clone()).unwrap_or_else(|| tr("(none)").to_string());
    crate::theme::modal(egui::Id::unique("payments")).show(ctx, |ui| {
        crate::theme::dialog_spacing(ui);
        ui.set_width(820.0);
        ui.label(RichText::new(tr("Payment Schedule")).strong().size(14.0));
        ui.add_space(6.0);
        egui::Grid::new("pay_terms").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Contract value:"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut st.plan.contract_value).range(0.0..=1e12).speed(1000.0).prefix(crate::util::currency_symbol()));
                ui.label(RichText::new(trf("0 = the project's total cost ({})", &[&crate::util::fmt_money(cost_total)])).color(theme::TEXT_DIM));
            });
            ui.end_row();
            ui.label(tr("Advance payment:"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut st.plan.advance_pct).range(0.0..=100.0).speed(0.5).suffix(" %"));
                ui.label(RichText::new(tr("recovered from each installment in the same proportion")).color(theme::TEXT_DIM));
            });
            ui.end_row();
            ui.label(tr("Retention:"));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut st.plan.retention_pct).range(0.0..=100.0).speed(0.5).suffix(" %"));
                ui.label(RichText::new(tr("withheld from each installment")).color(theme::TEXT_DIM));
            });
            ui.end_row();
        });
        ui.add_space(8.0);
        let contract = if st.plan.contract_value > 0.0 { st.plan.contract_value } else { cost_total };
        let mut remove = None;
        let mut up = None;
        egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("pay_items").striped(true).num_columns(7).spacing([8.0, 4.0]).show(ui, |ui| {
                for h in ["Installment", "Share of contract", "Fixed amount", "Due when this task is done", "Amount", "Paid", ""] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for (k, it) in st.plan.items.iter_mut().enumerate() {
                    ui.add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut it.name));
                    ui.add_enabled(it.amount <= 0.0, egui::DragValue::new(&mut it.percent).range(0.0..=100.0).speed(0.1).suffix(" %").max_decimals(2));
                    ui.add(egui::DragValue::new(&mut it.amount).range(0.0..=1e12).speed(1000.0).prefix(crate::util::currency_symbol()));
                    egui::ComboBox::from_id_salt(("pay_task", k)).width(250.0).selected_text(crate::thai::shape(&name_of(it.task_uid)).to_string()).show_ui(ui, |ui| {
                        ui.selectable_value(&mut it.task_uid, None, tr("(none)"));
                        for (u, n) in &choices {
                            ui.selectable_value(&mut it.task_uid, Some(*u), crate::thai::shape(n).to_string());
                        }
                    });
                    let amount = if it.amount > 0.0 { it.amount } else { contract * it.percent / 100.0 };
                    ui.label(crate::util::fmt_money(amount));
                    ui.checkbox(&mut it.paid, "");
                    ui.horizontal(|ui| {
                        if k > 0 && ui.small_button(tr("Move up")).clicked() {
                            up = Some(k);
                        }
                        if ui.small_button("×").clicked() {
                            remove = Some(k);
                        }
                    });
                    ui.end_row();
                }
            });
        });
        if let Some(k) = remove {
            st.plan.items.remove(k);
        }
        if let Some(k) = up {
            st.plan.items.swap(k, k - 1);
        }
        let pct_sum: f64 = st.plan.items.iter().map(|it| if it.amount > 0.0 && contract > 0.0 { it.amount / contract * 100.0 } else { it.percent }).sum();
        ui.horizontal(|ui| {
            if ui.button(tr("+ Add installment")).clicked() {
                let n = st.plan.items.len() + 1;
                let rest = (100.0 - pct_sum).max(0.0);
                st.plan.items.push(Payment { name: if thai { format!("งวดที่ {n}") } else { format!("Installment {n}") }, percent: (rest * 100.0).round() / 100.0, ..Default::default() });
            }
            if ui.button(tr("From milestones")).on_hover_text(tr("One installment per milestone, sized by the value of the work finished since the milestone before")).clicked() {
                let list = from_milestones(&app.doc, thai);
                if list.is_empty() {
                    st.error = Some(tr("The plan has no milestones.").to_string());
                } else {
                    st.plan.items = list;
                    st.error = None;
                }
            }
        });
        if !st.plan.items.is_empty() {
            let sum_text = trf("Total {} of the contract", &[&format!("{:.2}%", pct_sum.abs())]);
            let off = (pct_sum - 100.0).abs() > 0.01;
            ui.label(RichText::new(sum_text).color(if off { egui::Color32::from_rgb(0xC4, 0x31, 0x4B) } else { theme::TEXT_DIM }));
        }
        crate::dialogs::error_line(ui, &st.error);
        let (ok, cancel) = crate::dialogs::buttons(ui, "OK");
        if ok {
            done = Some(true);
        } else if cancel {
            done = Some(false);
        }
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        done = Some(false);
    }
    if done == Some(true) && app.doc.payments != st.plan {
        app.checkpoint();
        app.doc.payments = st.plan.clone();
    }
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Task;
    use chrono::Duration;

    /// The schedule survives .mpp, .xml and .neramit, still pointing at the same tasks.
    #[test]
    fn saved_in_every_format() {
        let mut p = Project::new();
        p.tasks.clear();
        let d = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        for (k, name) in ["เตรียมงาน", "ส่งมอบงวด 1", "งานหลังคา"].iter().enumerate() {
            let mut t = p.new_task(name);
            t.start = d + Duration::days(k as i64);
            t.fixed_cost = 1000.0 * (k + 1) as f64;
            p.tasks.push(t);
        }
        crate::schedule::schedule(&mut p).unwrap();
        let target = p.tasks[1].uid;
        p.payments = PaymentPlan {
            contract_value: 2_500_000.0,
            advance_pct: 15.0,
            retention_pct: 5.0,
            items: vec![Payment { name: "งวดที่ 1".into(), percent: 40.0, task_uid: Some(target), ..Default::default() }, Payment { name: "งวดที่ 2".into(), amount: 1_500_000.0, paid: true, ..Default::default() }],
        };
        let check = |q: &Project, how: &str| {
            assert_eq!(q.payments.contract_value, 2_500_000.0, "{how}");
            assert_eq!((q.payments.advance_pct, q.payments.retention_pct), (15.0, 5.0), "{how}");
            assert_eq!(q.payments.items.len(), 2, "{how}");
            let u = q.payments.items[0].task_uid.expect(how);
            assert_eq!(q.tasks.iter().find(|t| t.uid == u).map(|t| t.name.as_str()), Some("ส่งมอบงวด 1"), "{how}");
            assert_eq!((q.payments.items[1].amount, q.payments.items[1].paid, q.payments.items[1].task_uid), (1_500_000.0, true, None), "{how}");
        };
        let bytes = crate::mppt::save_new_mpp(&p).unwrap();
        check(&crate::mpp::load_mpp(&bytes).unwrap(), "mpp");
        let dir = std::env::temp_dir().join(format!("neramit-pay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for ext in ["xml", "neramit"] {
            let f = dir.join(format!("plan.{ext}"));
            crate::fileio::save(&f, &p).unwrap();
            check(&crate::fileio::load(&f).unwrap(), ext);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn installments_from_milestones() {
        let mut p = Project::new();
        p.tasks.clear();
        let d = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let mut add = |name: &str, day: i64, cost: f64, ms: bool| {
            let mut t = Task::new(p.next_uid, name, d + Duration::days(day));
            p.next_uid += 1;
            t.finish = t.start;
            t.cost = cost;
            t.milestone = ms;
            t.duration = if ms { 0 } else { 1 };
            p.tasks.push(t);
        };
        add("ฐานราก", 0, 300.0, false);
        add("ส่งมอบฐานราก", 1, 0.0, true);
        add("โครงสร้าง", 2, 500.0, false);
        add("หลังคา", 3, 200.0, false);
        add("ส่งมอบงาน", 4, 0.0, true);
        let list = from_milestones(&p, true);
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].name.as_str(), list[0].percent, list[1].percent), ("งวดที่ 1", 30.0, 70.0));
        assert_eq!(list[1].task_uid, Some(p.tasks[4].uid));
        // terms: 15 % advance and 5 % retention out of every installment
        p.payments = PaymentPlan { contract_value: 1_000_000.0, advance_pct: 15.0, retention_pct: 5.0, items: list };
        p.tasks[0].percent_complete = 100;
        p.tasks[1].percent_complete = 100;
        let s = schedule(&p, d + Duration::days(10));
        assert_eq!(s.advance_amount, 150_000.0);
        assert_eq!(s.rows[0].amount, 300_000.0);
        assert_eq!((s.rows[0].advance, s.rows[0].retention, s.rows[0].net), (45_000.0, 15_000.0, 240_000.0));
        assert_eq!(s.rows[0].status, Status::Due);
        assert_eq!(s.rows[1].status, Status::Late);
        assert!((s.pct_sum - 100.0).abs() < 1e-9);
        assert_eq!(s.due_net, 240_000.0);
        // a fixed amount wins over the percentage
        p.payments.items[1].amount = 650_000.0;
        p.payments.items[1].paid = true;
        let s = schedule(&p, d);
        assert!((s.rows[1].pct - 65.0).abs() < 1e-9);
        assert_eq!(s.rows[1].status, Status::Paid);
    }
}
