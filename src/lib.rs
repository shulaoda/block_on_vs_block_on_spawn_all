#![deny(clippy::all)]

use std::{future::Future, pin::Pin, sync::Arc};

use napi::{
  bindgen_prelude::{FnArgs, Promise},
  threadsafe_function::{ThreadsafeFunction, UnknownReturnValue},
  Either, Status,
};
use oxc::{
  ast::ast::Expression,
  ast_visit::{walk_mut, VisitMut},
  span::SourceType,
};

#[macro_use]
extern crate napi_derive;

pub type MaybeAsyncJsCallback<Args, Ret> = Arc<
  ThreadsafeFunction<
    Args,
    Either<Either<Promise<Ret>, Ret>, UnknownReturnValue>,
    Args,
    Status,
    false,
    true,
  >,
>;

pub type ResolverFn = dyn Fn(String, String) -> Pin<Box<(dyn Future<Output = anyhow::Result<Option<String>>> + Send)>>
  + Send
  + Sync;

const TESTS: [&str; 10] = ["TEST"; 10];
const TEST_JS: &str = include_str!("./test.js");

#[derive(Debug, Default)]
struct TestVisitor;
impl VisitMut<'_> for TestVisitor {
  fn visit_expression(&mut self, expr: &mut Expression<'_>) {
    walk_mut::walk_expression(self, expr);
    let _ = 1 + 1;
  }
}

pub async fn block_on_spawn_all<Iter, Out>(iter: Iter) -> Vec<Out>
where
  Iter: Iterator,
  Out: Send + 'static,
  Iter::Item: Future<Output = Out> + Send,
{
  let (_, collections) =
    async_scoped::Scope::scope_and_block(|scope: &mut async_scoped::TokioScope<'_, _>| {
      iter.into_iter().for_each(|fut| scope.spawn(fut));
    });
  collections.into_iter().map(Result::unwrap).collect()
}

pub fn block_on<F: Future>(f: F) -> F::Output {
  tokio::task::block_in_place(move || tokio::runtime::Handle::current().block_on(f))
}

#[napi]
pub async fn run_with_block(
  callback: MaybeAsyncJsCallback<FnArgs<(String, String)>, Option<String>>,
) -> Vec<String> {
  let allocator = oxc::allocator::Allocator::new();
  let mut parser_ret = oxc::parser::Parser::new(&allocator, TEST_JS, SourceType::mjs()).parse();
  let mut visitor = TestVisitor::default();
  visitor.visit_program(&mut parser_ret.program);
  let mut result = vec![];
  for test in TESTS {
    let future = callback.call_async((test.to_owned(), test.to_owned()).into());
    let ret = match block_on(future).unwrap() {
      Either::A(Either::A(promise)) => promise.await.unwrap(),
      Either::A(Either::B(ret)) => ret,
      Either::B(_) => continue,
    };
    if let Some(value) = ret {
      result.push(value);
    }
  }
  result
}

#[napi]
pub async fn run_with_join(
  callback: MaybeAsyncJsCallback<FnArgs<(String, String)>, Option<String>>,
) -> Vec<String> {
  let allocator = oxc::allocator::Allocator::new();
  let mut parser_ret = oxc::parser::Parser::new(&allocator, TEST_JS, SourceType::mjs()).parse();
  let mut visitor = TestVisitor::default();
  visitor.visit_program(&mut parser_ret.program);
  let futures = TESTS.into_iter().map(async |test| {
    match callback
      .call_async((test.to_owned(), test.to_owned()).into())
      .await
      .unwrap()
    {
      Either::A(Either::A(promise)) => promise.await.unwrap(),
      Either::A(Either::B(ret)) => ret,
      Either::B(_) => None,
    }
  });
  visitor.visit_program(&mut parser_ret.program);
  block_on_spawn_all(futures)
    .await
    .into_iter()
    .flatten()
    .collect()
}

#[napi]
pub async fn run_with_both(
  callback: MaybeAsyncJsCallback<FnArgs<(String, String)>, Option<String>>,
) -> Vec<String> {
  let allocator = oxc::allocator::Allocator::new();
  let mut parser_ret = oxc::parser::Parser::new(&allocator, TEST_JS, SourceType::mjs()).parse();
  let mut visitor = TestVisitor::default();
  visitor.visit_program(&mut parser_ret.program);
  let futures = TESTS.into_iter().map(async |test| {
    match callback
      .call_async((test.to_owned(), test.to_owned()).into())
      .await
      .unwrap()
    {
      Either::A(Either::A(promise)) => promise.await.unwrap(),
      Either::A(Either::B(ret)) => ret,
      Either::B(_) => None,
    }
  });
  block_on(block_on_spawn_all(futures))
    .into_iter()
    .flatten()
    .collect()
}
