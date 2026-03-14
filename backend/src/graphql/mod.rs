mod accounts;
mod journal;
mod subscriptions;
mod users;

use async_graphql::{EmptySubscription, MergedObject, Schema};

#[derive(MergedObject, Default)]
pub struct Query(
    users::UserQuery,
    accounts::AccountQuery,
    journal::JournalQuery,
);

#[derive(MergedObject, Default)]
pub struct Mutation(accounts::AccountMutation, journal::JournalMutation);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build_schema() -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription).finish()
}
