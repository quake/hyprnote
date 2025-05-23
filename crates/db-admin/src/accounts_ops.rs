use super::{Account, AdminDatabase};

impl AdminDatabase {
    pub async fn list_accounts(&self) -> Result<Vec<Account>, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn.query("SELECT * FROM accounts", ()).await?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await? {
            let item: Account = libsql::de::from_row(&row).unwrap();
            items.push(item);
        }
        Ok(items)
    }

    pub async fn upsert_account(&self, organization: Account) -> Result<Account, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn
            .query(
                "INSERT INTO accounts (
                    id,
                    turso_db_name,
                    clerk_org_id
                ) VALUES (?, ?, ?) RETURNING *",
                vec![
                    libsql::Value::Text(organization.id),
                    libsql::Value::Text(organization.turso_db_name),
                    organization
                        .clerk_org_id
                        .map(libsql::Value::Text)
                        .unwrap_or(libsql::Value::Null),
                ],
            )
            .await?;

        let row = rows.next().await?.unwrap();
        let org: Account = libsql::de::from_row(&row).unwrap();
        Ok(org)
    }

    pub async fn get_account_by_id(
        &self,
        id: impl Into<String>,
    ) -> Result<Option<Account>, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn
            .query("SELECT * FROM accounts WHERE id = ?", vec![id.into()])
            .await?;
        match rows.next().await? {
            None => Ok(None),
            Some(row) => {
                let org: Account = libsql::de::from_row(&row).unwrap();
                Ok(Some(org))
            }
        }
    }

    pub async fn get_account_by_clerk_org_id(
        &self,
        clerk_org_id: impl Into<String>,
    ) -> Result<Option<Account>, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn
            .query(
                "SELECT * FROM accounts WHERE clerk_org_id = ?",
                vec![clerk_org_id.into()],
            )
            .await?;

        match rows.next().await? {
            None => Ok(None),
            Some(row) => {
                let org: Account = libsql::de::from_row(&row).unwrap();
                Ok(Some(org))
            }
        }
    }

    pub async fn get_account_by_clerk_user_id(
        &self,
        clerk_user_id: impl Into<String>,
    ) -> Result<Option<Account>, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn
            .query(
                "SELECT o.* FROM accounts o
                 INNER JOIN users u ON u.account_id = o.id
                 WHERE u.clerk_user_id = ? AND o.clerk_org_id IS NULL",
                vec![clerk_user_id.into()],
            )
            .await?;

        match rows.next().await? {
            None => Ok(None),
            Some(row) => {
                let org: Account = libsql::de::from_row(&row).unwrap();
                Ok(Some(org))
            }
        }
    }

    pub async fn list_accounts_by_clerk_user_id(
        &self,
        user_id: impl Into<String>,
    ) -> Result<Vec<Account>, crate::Error> {
        let conn = self.conn()?;

        let mut rows = conn
            .query(
                "SELECT o.* FROM accounts o
                 INNER JOIN users u ON u.account_id = o.id
                 WHERE u.clerk_user_id = ?",
                vec![user_id.into()],
            )
            .await?;

        let mut accounts = Vec::new();
        while let Some(row) = rows.next().await? {
            let org: Account = libsql::de::from_row(&row).unwrap();
            accounts.push(org);
        }

        Ok(accounts)
    }
}

#[cfg(test)]
mod tests {
    use crate::{tests::setup_db, Account, User};

    #[tokio::test]
    async fn test_accounts() {
        let db = setup_db().await;

        let org = Account {
            id: uuid::Uuid::new_v4().to_string(),
            turso_db_name: "yujonglee".to_string(),
            clerk_org_id: Some("org_1".to_string()),
        };

        let org = db.upsert_account(org).await.unwrap();
        assert_eq!(org.clerk_org_id, Some("org_1".to_string()));
    }

    #[tokio::test]
    async fn test_list_accounts_by_clerk_user_id() {
        let db = setup_db().await;

        let accounts = [
            Account {
                id: uuid::Uuid::new_v4().to_string(),
                turso_db_name: "account1".to_string(),
                clerk_org_id: Some("org_1".to_string()),
            },
            Account {
                id: uuid::Uuid::new_v4().to_string(),
                turso_db_name: "account2".to_string(),
                clerk_org_id: Some("org_2".to_string()),
            },
        ];
        for account in accounts.clone() {
            db.upsert_account(account).await.unwrap();
        }

        let users = [
            User {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: accounts[0].id.clone(),
                human_id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                clerk_user_id: "clerk_user_123".to_string(),
            },
            User {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: accounts[1].id.clone(),
                human_id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                clerk_user_id: "clerk_user_456".to_string(),
            },
        ];
        for user in users {
            db.upsert_user(user).await.unwrap();
        }

        let accounts = db
            .list_accounts_by_clerk_user_id("clerk_user_123")
            .await
            .unwrap();
        assert_eq!(accounts.len(), 1);

        let accounts = db
            .list_accounts_by_clerk_user_id("clerk_user_456")
            .await
            .unwrap();
        assert_eq!(accounts.len(), 1);
    }
}
