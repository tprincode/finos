//! Managed external accounts. Charges are read from external_register_line.

use application_core::contracts::{
    ExternalManagedAccount, ExternalManagedAccountSave, ExternalManagedGetBody, ExternalRegisterLine,
    LoanVendorData, LoanVendorLine, LoanWeekRow,
};
use application_core::external_account::{
    line_applied, line_matches, next_due, paid_through_month, projected_split, AccountMatch,
    RegisterMatch,
};
use application_core::external_register::project_category_bucket;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("external_account", err.to_string())
}

struct StoredAccount {
    account_id: String,
    name: String,
    account_name: String,
    kind: String,
    charges_interest: bool,
    starting_minor: Option<i64>,
    current_minor: Option<i64>,
    payment_minor: Option<i64>,
    reduction_minor: Option<i64>,
    finance_minor: Option<i64>,
    paid_through: Option<String>,
    due_on: Option<String>,
    apr_ppm: Option<i64>,
    frequency: Option<String>,
    register_key: String,
    pay_process: Option<String>,
    linked_element_id: Option<String>,
    inactive: bool,
    legacy_vendor: Option<String>,
    legacy_amount_minor: Option<i64>,
}

struct StoredLine {
    line: ExternalRegisterLine,
    completed: bool,
    step_transfer: bool,
    step_billpay: bool,
    step_billpay_deposit: bool,
    step_pay: bool,
    step_withdrawal: bool,
}

fn map_account(row: &sqlx::sqlite::SqliteRow) -> Result<StoredAccount, PlatformError> {
    let name: String = row.try_get("name").map_err(map_sql)?;
    let account_name: String = row
        .try_get::<String, _>("account_name")
        .unwrap_or_default();
    let account_name = if account_name.trim().is_empty() {
        name.clone()
    } else {
        account_name
    };
    Ok(StoredAccount {
        account_id: row.try_get("account_id").map_err(map_sql)?,
        name,
        account_name,
        kind: row.try_get("kind").map_err(map_sql)?,
        charges_interest: row.try_get::<i64, _>("charges_interest").map_err(map_sql)? != 0,
        starting_minor: row.try_get("starting_minor").map_err(map_sql)?,
        current_minor: row.try_get("current_minor").map_err(map_sql)?,
        payment_minor: row.try_get("payment_minor").map_err(map_sql)?,
        reduction_minor: row.try_get("reduction_minor").map_err(map_sql)?,
        finance_minor: row.try_get("finance_minor").map_err(map_sql)?,
        paid_through: row.try_get("paid_through").map_err(map_sql)?,
        due_on: row.try_get("due_on").map_err(map_sql)?,
        apr_ppm: row.try_get("apr_ppm").map_err(map_sql)?,
        frequency: row.try_get("frequency").map_err(map_sql)?,
        register_key: row.try_get("register_key").map_err(map_sql)?,
        pay_process: row.try_get("pay_process").map_err(map_sql)?,
        linked_element_id: row.try_get("linked_element_id").map_err(map_sql)?,
        inactive: row.try_get::<i64, _>("inactive").unwrap_or(0) != 0,
        legacy_vendor: row.try_get("legacy_vendor").map_err(map_sql)?,
        legacy_amount_minor: row.try_get("legacy_amount_minor").map_err(map_sql)?,
    })
}

fn map_line(row: &sqlx::sqlite::SqliteRow) -> Result<StoredLine, PlatformError> {
    let line_id: String = row.try_get("line_id").map_err(map_sql)?;
    let line_id = Uuid::parse_str(&line_id)
        .map_err(|_| PlatformError::new("bad_line_id", "stored register line id is not a uuid"))?;
    let scale: i64 = row.try_get("scale").map_err(map_sql)?;
    let completed = row.try_get::<i64, _>("completed").map_err(map_sql)? != 0;
    let step_transfer = row.try_get::<i64, _>("step_transfer").map_err(map_sql)? != 0;
    let step_billpay = row.try_get::<i64, _>("step_billpay").map_err(map_sql)? != 0;
    let step_billpay_deposit = row.try_get::<i64, _>("step_billpay_deposit").map_err(map_sql)? != 0;
    let step_pay = row.try_get::<i64, _>("step_pay").map_err(map_sql)? != 0;
    let step_withdrawal = row.try_get::<i64, _>("step_withdrawal").map_err(map_sql)? != 0;
    let stored_category: String = row.try_get("category").map_err(map_sql)?;
    let stored_bucket: String = row.try_get("bucket").map_err(map_sql)?;
    let loan_name: String = row.try_get("loan_name").unwrap_or_default();
    let (category, bucket) = project_category_bucket(&stored_category, &stored_bucket);
    Ok(StoredLine {
        line: ExternalRegisterLine {
            line_id,
            source_row: row.try_get("source_row").map_err(map_sql)?,
            pay_type: row.try_get("pay_type").map_err(map_sql)?,
            occurred_on: row.try_get("occurred_on").map_err(map_sql)?,
            amount_minor: row.try_get("amount_minor").map_err(map_sql)?,
            scale: scale as u8,
            category,
            bucket,
            loan_name,
            vendor: row.try_get("vendor").map_err(map_sql)?,
            description: row.try_get("description").map_err(map_sql)?,
            true_up_on: row.try_get("true_up_on").map_err(map_sql)?,
            completed,
            step_transfer,
            step_billpay,
            step_billpay_deposit,
            step_pay,
            step_withdrawal,
            step_transfer_on: row.try_get("step_transfer_on").map_err(map_sql)?,
            step_billpay_on: row.try_get("step_billpay_on").map_err(map_sql)?,
            step_billpay_deposit_on: row.try_get("step_billpay_deposit_on").map_err(map_sql)?,
            step_pay_on: row.try_get("step_pay_on").map_err(map_sql)?,
            step_withdrawal_on: row.try_get("step_withdrawal_on").map_err(map_sql)?,
        },
        completed,
        step_transfer,
        step_billpay,
        step_billpay_deposit,
        step_pay,
        step_withdrawal,
    })
}

async fn load_accounts(
    conn: &mut sqlx::SqliteConnection,
) -> Result<Vec<StoredAccount>, PlatformError> {
    let rows = sqlx::query(
        "SELECT account_id, name, account_name, kind, charges_interest, starting_minor, current_minor,
                payment_minor, reduction_minor, finance_minor, paid_through, due_on, apr_ppm,
                frequency, register_key, pay_process, linked_element_id, inactive,
                legacy_vendor, legacy_amount_minor
         FROM external_managed_account
         ORDER BY sort_order, name",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(map_sql)?;
    rows.iter().map(map_account).collect()
}

async fn load_lines(conn: &mut sqlx::SqliteConnection) -> Result<Vec<StoredLine>, PlatformError> {
    let rows = sqlx::query(
        "SELECT line_id, source_row, pay_type, occurred_on, amount_minor, scale,
                category, bucket, loan_name, vendor, description, true_up_on, completed,
                step_transfer, step_billpay, step_billpay_deposit, step_pay, step_withdrawal,
                step_transfer_on, step_billpay_on, step_billpay_deposit_on, step_pay_on,
                step_withdrawal_on
         FROM external_register_line",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(map_sql)?;
    rows.iter().map(map_line).collect()
}

async fn applied_ids(
    conn: &mut sqlx::SqliteConnection,
    account_id: &str,
) -> Result<std::collections::HashSet<String>, PlatformError> {
    let rows = sqlx::query("SELECT line_id FROM external_managed_applied WHERE account_id = ?1")
        .bind(account_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(map_sql)?;
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        ids.insert(row.try_get::<String, _>("line_id").map_err(map_sql)?);
    }
    Ok(ids)
}

fn account_match(account: &StoredAccount) -> AccountMatch {
    AccountMatch {
        kind: account.kind.clone(),
        register_key: account.register_key.clone(),
        legacy_vendor: account.legacy_vendor.clone(),
        legacy_amount_minor: account.legacy_amount_minor,
    }
}

fn register_match(line: &StoredLine) -> RegisterMatch {
    RegisterMatch {
        category: line.line.category.clone(),
        bucket: line.line.bucket.clone(),
        loan_name: line.line.loan_name.clone(),
        vendor: line.line.vendor.clone(),
        amount_minor: line.line.amount_minor,
    }
}

pub async fn get(pool: &SqlitePool) -> Result<ExternalManagedGetBody, PlatformError> {
    let mut tx = pool.begin().await.map_err(map_sql)?;
    reconcile(&mut tx).await?;
    let body = read_body(&mut tx).await?;
    tx.commit().await.map_err(map_sql)?;
    Ok(body)
}

async fn reconcile(conn: &mut sqlx::SqliteConnection) -> Result<(), PlatformError> {
    let accounts = load_accounts(conn).await?;
    let lines = load_lines(conn).await?;
    for account in accounts {
        // Inactive loans keep history but must not keep moving Current from CCT.
        if account.inactive {
            continue;
        }
        // Escrow (Mom): Bucket Transaction Managed. Debts: CCT settle via loan_name
        // (register legacy + scheduled element).
        let process = account.pay_process.as_deref();
        let settle_via_cct = if account.kind == "credit" {
            process == Some("register")
        } else {
            matches!(process, Some("register") | Some("element") | None)
        };
        if !settle_via_cct {
            continue;
        }
        let matcher = account_match(&account);
        let mut matched: Vec<&StoredLine> = lines
            .iter()
            .filter(|line| line_matches(&register_match(line), &matcher))
            .collect();
        matched.sort_by(|left, right| {
            left.line
                .occurred_on
                .as_deref()
                .unwrap_or("")
                .cmp(right.line.occurred_on.as_deref().unwrap_or(""))
        });
        let applied = applied_ids(conn, &account.account_id).await?;
        let mut current = account.current_minor;
        let mut reduction = account.reduction_minor;
        let mut paid_through = account.paid_through.clone();
        let mut changed = false;
        for line in matched {
            let ready = line_applied(
                &account.kind,
                line.completed,
                line.step_transfer,
                line.step_billpay,
                line.step_billpay_deposit,
                line.step_pay,
                line.step_withdrawal,
            );
            if !ready {
                continue;
            }
            let line_id = line.line.line_id.to_string();
            if applied.contains(&line_id) {
                continue;
            }
            if current.is_none() {
                continue;
            }
            if let Some(balance) = current.as_mut() {
                // Positive amount lowers Current; negative raises (loan charge or escrow deposit).
                *balance -= line.line.amount_minor;
            }
            if account.kind != "credit" {
                reduction = Some(line.line.amount_minor);
                if let Some(on) = line
                    .line
                    .occurred_on
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .or_else(|| line.line.true_up_on.as_deref().filter(|s| !s.is_empty()))
                {
                    if let Some(month) = paid_through_month(on) {
                        paid_through = Some(month.to_string());
                    }
                }
            }
            sqlx::query(
                "INSERT OR IGNORE INTO external_managed_applied (account_id, line_id) VALUES (?1, ?2)",
            )
            .bind(&account.account_id)
            .bind(&line_id)
            .execute(&mut *conn)
            .await
            .map_err(map_sql)?;
            changed = true;
        }
        if changed {
            sqlx::query(
                "UPDATE external_managed_account
                 SET current_minor = ?1, reduction_minor = ?2, paid_through = ?3
                 WHERE account_id = ?4",
            )
            .bind(current)
            .bind(reduction)
            .bind(&paid_through)
            .bind(&account.account_id)
            .execute(&mut *conn)
            .await
            .map_err(map_sql)?;
        }
    }
    Ok(())
}

async fn load_vendor(
    conn: &mut sqlx::SqliteConnection,
    account_id: &str,
) -> Result<Option<LoanVendorData>, PlatformError> {
    let header = sqlx::query(
        "SELECT term_payments, payments_made, principal_paid_minor, interest_paid_minor,
                escrow_paid_minor, principal_balance_minor, escrow_balance_minor, note
         FROM external_loan_vendor
         WHERE account_id = ?1",
    )
    .bind(account_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(map_sql)?;
    let Some(header) = header else {
        return Ok(None);
    };
    let rows = sqlx::query(
        "SELECT line_id, occurred_on, amount_minor, due_on, description,
                principal_minor, interest_minor, escrow_minor, late_minor,
                principal_balance_minor, escrow_balance_minor
         FROM external_loan_vendor_line
         WHERE account_id = ?1
         ORDER BY sort_order",
    )
    .bind(account_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(map_sql)?;
    let mut lines = Vec::new();
    for row in rows {
        let line_id: String = row.try_get("line_id").map_err(map_sql)?;
        let line_id = Uuid::parse_str(&line_id).map_err(|_| {
            PlatformError::new("bad_line_id", "stored loan vendor line id is not a uuid")
        })?;
        lines.push(LoanVendorLine {
            line_id,
            occurred_on: row.try_get("occurred_on").map_err(map_sql)?,
            amount_minor: row.try_get("amount_minor").map_err(map_sql)?,
            due_on: row.try_get("due_on").map_err(map_sql)?,
            description: row.try_get("description").map_err(map_sql)?,
            principal_minor: row.try_get("principal_minor").map_err(map_sql)?,
            interest_minor: row.try_get("interest_minor").map_err(map_sql)?,
            escrow_minor: row.try_get("escrow_minor").map_err(map_sql)?,
            late_minor: row.try_get("late_minor").map_err(map_sql)?,
            principal_balance_minor: row.try_get("principal_balance_minor").map_err(map_sql)?,
            escrow_balance_minor: row.try_get("escrow_balance_minor").map_err(map_sql)?,
        });
    }
    Ok(Some(LoanVendorData {
        term_payments: header.try_get("term_payments").map_err(map_sql)?,
        payments_made: header.try_get("payments_made").map_err(map_sql)?,
        principal_paid_minor: header.try_get("principal_paid_minor").map_err(map_sql)?,
        interest_paid_minor: header.try_get("interest_paid_minor").map_err(map_sql)?,
        escrow_paid_minor: header.try_get("escrow_paid_minor").map_err(map_sql)?,
        principal_balance_minor: header.try_get("principal_balance_minor").map_err(map_sql)?,
        escrow_balance_minor: header.try_get("escrow_balance_minor").map_err(map_sql)?,
        note: header.try_get("note").map_err(map_sql)?,
        lines,
    }))
}

async fn read_body(conn: &mut sqlx::SqliteConnection) -> Result<ExternalManagedGetBody, PlatformError> {
    let accounts = load_accounts(conn).await?;
    let lines = load_lines(conn).await?;
    let mut out = Vec::new();
    for account in accounts {
        let matcher = account_match(&account);
        let applied = applied_ids(conn, &account.account_id).await?;
        let mut matched: Vec<&StoredLine> = lines
            .iter()
            .filter(|line| line_matches(&register_match(line), &matcher))
            .collect();
        let mut pending_minor = 0_i64;
        for line in &matched {
            let ready = line_applied(
                &account.kind,
                line.completed,
                line.step_transfer,
                line.step_billpay,
                line.step_billpay_deposit,
                line.step_pay,
                line.step_withdrawal,
            );
            let already = applied.contains(&line.line.line_id.to_string());
            if !ready || (account.current_minor.is_none() && !already) {
                pending_minor += line.line.amount_minor;
            }
        }
        matched.sort_by(|left, right| {
            let left_ready = line_applied(
                &account.kind,
                left.completed,
                left.step_transfer,
                left.step_billpay,
                left.step_billpay_deposit,
                left.step_pay,
                left.step_withdrawal,
            );
            let right_ready = line_applied(
                &account.kind,
                right.completed,
                right.step_transfer,
                right.step_billpay,
                right.step_billpay_deposit,
                right.step_pay,
                right.step_withdrawal,
            );
            left_ready
                .cmp(&right_ready)
                .then_with(|| {
                    right
                        .line
                        .occurred_on
                        .as_deref()
                        .unwrap_or("")
                        .cmp(left.line.occurred_on.as_deref().unwrap_or(""))
                })
        });
        let account_id = Uuid::parse_str(&account.account_id).map_err(|_| {
            PlatformError::new("bad_account_id", "stored managed account id is not a uuid")
        })?;
        let linked_element_id = match account.linked_element_id.as_deref() {
            Some(value) if !value.is_empty() => Some(Uuid::parse_str(value).map_err(|_| {
                PlatformError::new("bad_element_id", "stored element link is not a uuid")
            })?),
            _ => None,
        };
        let vendor = load_vendor(conn, &account.account_id).await?;
        out.push(ExternalManagedAccount {
            account_id,
            name: account.name,
            account_name: account.account_name,
            kind: account.kind,
            charges_interest: account.charges_interest,
            starting_minor: account.starting_minor,
            current_minor: account.current_minor,
            payment_minor: account.payment_minor,
            reduction_minor: account.reduction_minor,
            pending_minor,
            finance_minor: account.finance_minor,
            paid_through: account.paid_through,
            due_on: account.due_on,
            apr_ppm: account.apr_ppm,
            frequency: account.frequency,
            register_key: account.register_key,
            pay_process: account.pay_process,
            linked_element_id,
            inactive: account.inactive,
            lines: matched.into_iter().map(|line| line.line.clone()).collect(),
            vendor,
        });
    }
    Ok(ExternalManagedGetBody { accounts: out })
}

pub async fn save(
    pool: &SqlitePool,
    accounts: Vec<ExternalManagedAccountSave>,
) -> Result<ExternalManagedGetBody, PlatformError> {
    let mut tx = pool.begin().await.map_err(map_sql)?;
    let stored = load_accounts(&mut tx).await?;
    let lines = load_lines(&mut tx).await?;
    for incoming in accounts {
        let account_id = incoming.account_id.to_string();
        let Some(existing) = stored.iter().find(|row| row.account_id == account_id) else {
            insert_account(&mut tx, &incoming).await?;
            continue;
        };
        let account_name = if incoming.account_name.trim().is_empty() {
            incoming.name.clone()
        } else {
            incoming.account_name.trim().to_string()
        };
        let register_key = if existing.kind == "credit" {
            existing.register_key.clone()
        } else if !incoming.register_key.trim().is_empty() {
            incoming.register_key.trim().to_string()
        } else {
            incoming.name.clone()
        };
        let finance = if existing.charges_interest || incoming.apr_ppm.unwrap_or(0) > 0 {
            incoming.finance_minor.or(existing.finance_minor)
        } else {
            None
        };
        let charges_interest = match incoming.apr_ppm {
            Some(rate) if rate > 0 => true,
            Some(_) => false,
            None => existing.charges_interest,
        };
        let mut current_minor = incoming.current_minor;
        if existing.kind == "credit" && current_minor.is_none() {
            current_minor = incoming.starting_minor;
        }
        let balance_edited = current_minor != existing.current_minor
            || incoming.starting_minor != existing.starting_minor;
        let (pay_process, linked_element_id) = if existing.kind == "credit" {
            ("register".to_string(), None)
        } else {
            (incoming.pay_process.clone(), incoming.linked_element_id)
        };
        if pay_process == "element" {
            if let Some(element_id) = linked_element_id {
                require_element(&mut tx, &element_id.to_string()).await?;
            }
        }
        let inactive = if existing.kind == "credit" {
            false
        } else {
            incoming.inactive
        };
        sqlx::query(
            "UPDATE external_managed_account
             SET name = ?1,
                 account_name = ?2,
                 starting_minor = ?3,
                 current_minor = ?4,
                 payment_minor = ?5,
                 reduction_minor = ?6,
                 finance_minor = ?7,
                 due_on = ?8,
                 apr_ppm = ?9,
                 frequency = ?10,
                 charges_interest = ?11,
                 register_key = ?12,
                 pay_process = ?13,
                 linked_element_id = ?14,
                 inactive = ?15
             WHERE account_id = ?16",
        )
        .bind(&incoming.name)
        .bind(&account_name)
        .bind(incoming.starting_minor)
        .bind(current_minor)
        .bind(incoming.payment_minor)
        .bind(incoming.reduction_minor)
        .bind(finance)
        .bind(&incoming.due_on)
        .bind(incoming.apr_ppm)
        .bind(&incoming.frequency)
        .bind(i64::from(charges_interest))
        .bind(&register_key)
        .bind(&pay_process)
        .bind(linked_element_id.map(|id| id.to_string()))
        .bind(i64::from(inactive))
        .bind(&account_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
        if existing.kind == "credit" && balance_edited {
            let matcher = AccountMatch {
                kind: existing.kind.clone(),
                register_key: existing.register_key.clone(),
                legacy_vendor: existing.legacy_vendor.clone(),
                legacy_amount_minor: existing.legacy_amount_minor,
            };
            for line in &lines {
                if !line_matches(&register_match(line), &matcher) {
                    continue;
                }
                if !line_applied(
                    &existing.kind,
                    line.completed,
                    line.step_transfer,
                    line.step_billpay,
                    line.step_billpay_deposit,
                    line.step_pay,
                    line.step_withdrawal,
                ) {
                    continue;
                }
                sqlx::query(
                    "INSERT OR IGNORE INTO external_managed_applied (account_id, line_id) VALUES (?1, ?2)",
                )
                .bind(&account_id)
                .bind(line.line.line_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(map_sql)?;
            }
        }
    }
    reconcile(&mut tx).await?;
    let body = read_body(&mut tx).await?;
    tx.commit().await.map_err(map_sql)?;
    Ok(body)
}
pub async fn loans_due(
    pool: &SqlitePool,
    start: &str,
    end: &str,
) -> Result<Vec<LoanWeekRow>, PlatformError> {
    let rows = sqlx::query(
        "SELECT account_id, name, charges_interest, current_minor, payment_minor,
                due_on, apr_ppm, frequency
         FROM external_managed_account
         WHERE kind = 'debt'
           AND pay_process = 'week_ahead'
           AND COALESCE(inactive, 0) = 0
           AND due_on IS NOT NULL
           AND due_on >= ?1
           AND due_on <= ?2
           AND frequency IS NOT NULL
           AND payment_minor IS NOT NULL
           AND payment_minor > 0
           AND NOT EXISTS (
               SELECT 1 FROM external_loan_payment p
               WHERE p.account_id = external_managed_account.account_id
                 AND p.due_on = external_managed_account.due_on
           )
         ORDER BY due_on, name",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    let mut loans = Vec::new();
    for row in rows {
        let account_id: String = row.try_get("account_id").map_err(map_sql)?;
        let account_id = Uuid::parse_str(&account_id).map_err(|_| {
            PlatformError::new("bad_account_id", "stored managed account id is not a uuid")
        })?;
        let due_on: String = row.try_get("due_on").map_err(map_sql)?;
        let payment_minor: i64 = row.try_get("payment_minor").map_err(map_sql)?;
        let balance: Option<i64> = row.try_get("current_minor").map_err(map_sql)?;
        let charges_interest = row.try_get::<i64, _>("charges_interest").map_err(map_sql)? != 0;
        let apr_ppm: Option<i64> = row.try_get("apr_ppm").map_err(map_sql)?;
        let frequency: Option<String> = row.try_get("frequency").map_err(map_sql)?;
        let (principal, interest) = projected_split(
            balance.unwrap_or(0),
            payment_minor,
            charges_interest,
            apr_ppm,
            frequency.as_deref(),
        )
        .unwrap_or((payment_minor, 0));
        loans.push(LoanWeekRow {
            account_id,
            name: row.try_get("name").map_err(map_sql)?,
            due_on,
            payment_minor,
            principal_minor: principal,
            interest_minor: interest,
            scale: 2,
        });
    }
    Ok(loans)
}

pub async fn confirm_loan_payment(
    pool: &SqlitePool,
    account_id: Uuid,
    due_on: String,
    principal_minor: i64,
    interest_minor: i64,
) -> Result<ExternalManagedGetBody, PlatformError> {
    if principal_minor < 0 || interest_minor < 0 {
        return Err(PlatformError::new(
            "bad_amount",
            "principal and interest stay zero or positive",
        ));
    }
    let mut tx = pool.begin().await.map_err(map_sql)?;
    let row = sqlx::query(
        "SELECT frequency, due_on, current_minor, pay_process
         FROM external_managed_account WHERE account_id = ?1",
    )
    .bind(account_id.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sql)?
    .ok_or_else(|| PlatformError::new("unknown_account", "that loan is not on the planner"))?;
    let pay_process: Option<String> = row.try_get("pay_process").map_err(map_sql)?;
    if pay_process.as_deref() != Some("week_ahead") {
        return Err(PlatformError::new(
            "not_week_ahead",
            "only a Week Ahead loan confirms on this row",
        ));
    }
    let stored_due: Option<String> = row.try_get("due_on").map_err(map_sql)?;
    if stored_due.as_deref() != Some(due_on.as_str()) {
        return Err(PlatformError::new(
            "stale_due",
            "that due date is no longer the open payment",
        ));
    }
    let frequency: Option<String> = row.try_get("frequency").map_err(map_sql)?;
    let next = next_due(due_on.as_str(), frequency.as_deref().unwrap_or("")).ok_or_else(|| {
        PlatformError::new("missing_frequency", "the loan needs a frequency before confirm")
    })?;
    let current: Option<i64> = row.try_get("current_minor").map_err(map_sql)?;
    let reduced = current.map(|balance| (balance - principal_minor).max(0));
    sqlx::query(
        "INSERT INTO external_loan_payment (
            payment_id, account_id, due_on, principal_minor, interest_minor
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(account_id.to_string())
    .bind(&due_on)
    .bind(principal_minor)
    .bind(interest_minor)
    .execute(&mut *tx)
    .await
    .map_err(map_sql)?;
    sqlx::query(
        "UPDATE external_managed_account
         SET current_minor = ?1, paid_through = ?2, due_on = ?3
         WHERE account_id = ?4",
    )
    .bind(reduced)
    .bind(&due_on)
    .bind(&next)
    .bind(account_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(map_sql)?;
    let body = read_body(&mut tx).await?;
    tx.commit().await.map_err(map_sql)?;
    Ok(body)
}

async fn require_element(conn: &mut sqlx::SqliteConnection, element_id: &str) -> Result<(), PlatformError> {
    let found = sqlx::query("SELECT element_id FROM cash_element WHERE element_id = ?1")
        .bind(element_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sql)?;
    if found.is_none() {
        return Err(PlatformError::new(
            "unknown_element",
            "that element is not on the element list",
        ));
    }
    Ok(())
}

async fn insert_account(
    conn: &mut sqlx::SqliteConnection,
    incoming: &application_core::contracts::ExternalManagedAccountSave,
) -> Result<(), PlatformError> {
    let name_taken = sqlx::query("SELECT account_id FROM external_managed_account WHERE name = ?1")
        .bind(&incoming.name)
        .fetch_optional(&mut *conn)
        .await
        .map_err(map_sql)?;
    if name_taken.is_some() {
        return Err(PlatformError::new(
            "duplicate_name",
            "a loan with that name is already on the list",
        ));
    }
    let linked = if incoming.pay_process == "element" {
        if let Some(element_id) = incoming.linked_element_id {
            require_element(conn, &element_id.to_string()).await?;
            Some(element_id.to_string())
        } else {
            None
        }
    } else {
        None
    };
    let charges_interest = incoming.apr_ppm.unwrap_or(0) > 0;
    let account_name = if incoming.account_name.trim().is_empty() {
        incoming.name.clone()
    } else {
        incoming.account_name.trim().to_string()
    };
    let register_key = if !incoming.register_key.trim().is_empty() {
        incoming.register_key.trim().to_string()
    } else {
        incoming.name.clone()
    };
    let sort_order: i64 =
        sqlx::query("SELECT COALESCE(MAX(sort_order), 0) + 1 AS next_sort FROM external_managed_account")
            .fetch_one(&mut *conn)
            .await
            .map_err(map_sql)?
            .try_get("next_sort")
            .map_err(map_sql)?;
    sqlx::query(
        "INSERT INTO external_managed_account (
            account_id, name, account_name, kind, charges_interest,
            starting_minor, current_minor, payment_minor, reduction_minor, finance_minor,
            paid_through, due_on, apr_ppm, frequency, register_key,
            pay_process, linked_element_id, inactive, sort_order
         ) VALUES (?1, ?2, ?3, 'debt', ?4, ?5, ?6, ?7, NULL, ?8, NULL, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    )
    .bind(incoming.account_id.to_string())
    .bind(&incoming.name)
    .bind(&account_name)
    .bind(i64::from(charges_interest))
    .bind(incoming.starting_minor)
    .bind(incoming.current_minor)
    .bind(incoming.payment_minor)
    .bind(if charges_interest { incoming.finance_minor } else { None })
    .bind(&incoming.due_on)
    .bind(incoming.apr_ppm)
    .bind(&incoming.frequency)
    .bind(&register_key)
    .bind(&incoming.pay_process)
    .bind(linked)
    .bind(i64::from(incoming.inactive))
    .bind(sort_order)
    .execute(&mut *conn)
    .await
    .map_err(map_sql)?;
    Ok(())
}

/// Week Ahead confirm for a Loan Element: draft an Open CCT row with Loan Name.
/// Loan Current moves when that CCT row settles (reconcile), not here.
pub async fn apply_element_payment(
    pool: &SqlitePool,
    element_id: Uuid,
    occurrence_id: Uuid,
    occurred_on: String,
    amount_minor: i64,
) -> Result<(), PlatformError> {
    draft_cct_from_loan_element(pool, element_id, occurrence_id, occurred_on, amount_minor).await
}

fn suggested_category_for_loan(loan_name: &str) -> &'static str {
    let key = loan_name.trim().to_lowercase();
    if key.contains("alphaeon") || key.contains("paytient") || key.contains("uva") {
        "Medical"
    } else if key.contains("newrez") {
        "Bills"
    } else {
        "Cash"
    }
}

pub async fn draft_cct_from_loan_element(
    pool: &SqlitePool,
    element_id: Uuid,
    occurrence_id: Uuid,
    occurred_on: String,
    amount_minor: i64,
) -> Result<(), PlatformError> {
    let mut tx = pool.begin().await.map_err(map_sql)?;
    let already = sqlx::query(
        "SELECT line_id FROM external_element_cct_draft WHERE occurrence_id = ?1",
    )
    .bind(occurrence_id.to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_sql)?;
    if already.is_some() {
        tx.commit().await.map_err(map_sql)?;
        return Ok(());
    }
    let rows = sqlx::query(
        "SELECT account_id, name, register_key, payment_minor, legacy_vendor
         FROM external_managed_account
         WHERE kind = 'debt'
           AND pay_process = 'element'
           AND linked_element_id = ?1",
    )
    .bind(element_id.to_string())
    .fetch_all(&mut *tx)
    .await
    .map_err(map_sql)?;
    for row in rows {
        let account_id: String = row.try_get("account_id").map_err(map_sql)?;
        let name: String = row.try_get("name").map_err(map_sql)?;
        let register_key: String = row.try_get("register_key").map_err(map_sql)?;
        let loan_name = if !register_key.trim().is_empty() {
            register_key
        } else {
            name.clone()
        };
        let payment: Option<i64> = row.try_get("payment_minor").map_err(map_sql)?;
        let vendor: Option<String> = row.try_get("legacy_vendor").map_err(map_sql)?;
        let payment_minor = payment.filter(|amount| *amount > 0).unwrap_or(amount_minor);
        if payment_minor <= 0 {
            continue;
        }
        let line_id = Uuid::new_v4();
        let category = suggested_category_for_loan(&loan_name);
        let vendor = vendor.unwrap_or_default();
        sqlx::query(
            "INSERT INTO external_register_line (
                line_id, source_row, pay_type, occurred_on, amount_minor, scale,
                category, bucket, loan_name, vendor, description, true_up_on, completed
             ) VALUES (?1, NULL, 'Checking', ?2, ?3, 2, ?4, '', ?5, ?6, ?7, NULL, 0)",
        )
        .bind(line_id.to_string())
        .bind(&occurred_on)
        .bind(payment_minor)
        .bind(category)
        .bind(&loan_name)
        .bind(&vendor)
        .bind(format!("Week Ahead loan · {loan_name}"))
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
        sqlx::query(
            "INSERT INTO external_element_cct_draft (occurrence_id, line_id, account_id)
             VALUES (?1, ?2, ?3)",
        )
        .bind(occurrence_id.to_string())
        .bind(line_id.to_string())
        .bind(&account_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sql)?;
    }
    tx.commit().await.map_err(map_sql)?;
    Ok(())
}
