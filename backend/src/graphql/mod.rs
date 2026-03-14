mod accounts;
mod analytics;
mod journal;
mod notebook;
mod subscriptions;
mod users;

use async_graphql::{EmptySubscription, MergedObject, Schema};

#[derive(MergedObject, Default)]
pub struct Query(
    users::UserQuery,
    accounts::AccountQuery,
    analytics::AnalyticsQuery,
    journal::JournalQuery,
    notebook::NotebookQuery,
);

#[derive(MergedObject, Default)]
pub struct Mutation(
    accounts::AccountMutation,
    journal::JournalMutation,
    notebook::NotebookMutation,
);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build_schema() -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription).finish()
}
