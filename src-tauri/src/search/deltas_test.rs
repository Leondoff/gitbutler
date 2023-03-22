use crate::{
    deltas::{self, Operation},
    projects, repositories,
};
use anyhow::Result;
use core::ops::Range;
use std::path::Path;
use tempfile::tempdir;

fn test_project() -> Result<repositories::Repository> {
    let path = tempdir()?.path().to_str().unwrap().to_string();
    std::fs::create_dir_all(&path)?;
    let repo = git2::Repository::init(&path)?;
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let sig = git2::Signature::now("test", "test@email.com").unwrap();
    let _commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "initial commit",
        &repo.find_tree(oid)?,
        &[],
    )?;
    let project = projects::Project::from_path(path)?;
    repositories::Repository::new(project, None)
}

#[test]
fn test_filter_by_timestamp() {
    let repository = test_project().unwrap();
    let index_path = tempdir().unwrap().path().to_str().unwrap().to_string();

    let mut session = repository.sessions_storage.create_current().unwrap();
    repository
        .deltas_storage
        .write(
            Path::new("test.txt"),
            &vec![
                deltas::Delta {
                    operations: vec![Operation::Insert((0, "Hello".to_string()))],
                    timestamp_ms: 0,
                },
                deltas::Delta {
                    operations: vec![Operation::Insert((5, "World".to_string()))],
                    timestamp_ms: 1,
                },
                deltas::Delta {
                    operations: vec![Operation::Insert((5, " ".to_string()))],
                    timestamp_ms: 2,
                },
            ],
        )
        .unwrap();
    session = repository.sessions_storage.flush(&session, None).unwrap();

    let mut searcher = super::Deltas::at(index_path.into()).unwrap();

    let write_result = searcher.index_session(&repository, &session);
    assert!(write_result.is_ok());

    let search_result_from = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "test.txt".to_string(),
        limit: 10,
        range: Range { start: 2, end: 10 },
        offset: None,
    });
    assert!(search_result_from.is_ok());
    let search_result_from = search_result_from.unwrap();
    assert_eq!(search_result_from.len(), 1);
    assert_eq!(search_result_from[0].index, 2);

    let search_result_to = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "test.txt".to_string(),
        limit: 10,
        range: Range { start: 0, end: 1 },
        offset: None,
    });
    assert!(search_result_to.is_ok());
    let search_result_to = search_result_to.unwrap();
    assert_eq!(search_result_to.len(), 1);
    assert_eq!(search_result_to[0].index, 0);

    let search_result_from_to = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "test.txt".to_string(),
        limit: 10,
        range: Range { start: 1, end: 2 },
        offset: None,
    });
    assert!(search_result_from_to.is_ok());
    let search_result_from_to = search_result_from_to.unwrap();
    assert_eq!(search_result_from_to.len(), 1);
    assert_eq!(search_result_from_to[0].index, 1);
}

#[test]
fn test_sorted_by_timestamp() {
    let repository = test_project().unwrap();
    let index_path = tempdir().unwrap().path().to_str().unwrap().to_string();

    let mut session = repository.sessions_storage.create_current().unwrap();
    repository
        .deltas_storage
        .write(
            Path::new("test.txt"),
            &vec![
                deltas::Delta {
                    operations: vec![Operation::Insert((0, "Hello".to_string()))],
                    timestamp_ms: 0,
                },
                deltas::Delta {
                    operations: vec![Operation::Insert((5, " World".to_string()))],
                    timestamp_ms: 1,
                },
            ],
        )
        .unwrap();
    session = repository.sessions_storage.flush(&session, None).unwrap();

    let mut searcher = super::Deltas::at(index_path.into()).unwrap();

    let write_result = searcher.index_session(&repository, &session);
    assert!(write_result.is_ok());

    let search_result = searcher.search(&super::SearchQuery {
        project_id: repository.project.id,
        q: "hello world".to_string(),
        limit: 10,
        range: Range { start: 0, end: 10 },
        offset: None,
    });
    assert!(search_result.is_ok());
    let search_result = search_result.unwrap();
    println!("{:?}", search_result);
    assert_eq!(search_result.len(), 2);
    assert_eq!(search_result[0].index, 1);
    assert_eq!(search_result[1].index, 0);
}

#[test]
fn test_simple() {
    let repository = test_project().unwrap();
    let index_path = tempdir().unwrap().path().to_str().unwrap().to_string();

    let mut session = repository.sessions_storage.create_current().unwrap();
    repository
        .deltas_storage
        .write(
            Path::new("test.txt"),
            &vec![
                deltas::Delta {
                    operations: vec![Operation::Insert((0, "Hello".to_string()))],
                    timestamp_ms: 0,
                },
                deltas::Delta {
                    operations: vec![Operation::Insert((5, " World".to_string()))],
                    timestamp_ms: 0,
                },
            ],
        )
        .unwrap();
    session = repository.sessions_storage.flush(&session, None).unwrap();

    let mut searcher = super::Deltas::at(index_path.into()).unwrap();

    let write_result = searcher.index_session(&repository, &session);
    assert!(write_result.is_ok());

    let search_result1 = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "hello".to_string(),
        limit: 10,
        offset: None,
        range: Range { start: 0, end: 10 },
    });
    println!("{:?}", search_result1);
    assert!(search_result1.is_ok());
    let search_result1 = search_result1.unwrap();
    assert_eq!(search_result1.len(), 1);
    assert_eq!(search_result1[0].session_id, session.id);
    assert_eq!(search_result1[0].project_id, repository.project.id);
    assert_eq!(search_result1[0].file_path, "test.txt");
    assert_eq!(search_result1[0].index, 0);

    let search_result2 = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "world".to_string(),
        limit: 10,
        offset: None,
        range: Range { start: 0, end: 10 },
    });
    assert!(search_result2.is_ok());
    let search_result2 = search_result2.unwrap();
    assert_eq!(search_result2.len(), 1);
    assert_eq!(search_result2[0].session_id, session.id);
    assert_eq!(search_result2[0].project_id, repository.project.id);
    assert_eq!(search_result2[0].file_path, "test.txt");
    assert_eq!(search_result2[0].index, 1);

    let search_result3 = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "hello world".to_string(),
        limit: 10,
        offset: None,
        range: Range { start: 0, end: 10 },
    });
    assert!(search_result3.is_ok());
    let search_result3 = search_result3.unwrap();
    assert_eq!(search_result3.len(), 2);
    assert_eq!(search_result3[0].project_id, repository.project.id);
    assert_eq!(search_result3[0].session_id, session.id);
    assert_eq!(search_result3[0].file_path, "test.txt");
    assert_eq!(search_result3[1].session_id, session.id);
    assert_eq!(search_result3[1].project_id, repository.project.id);
    assert_eq!(search_result3[1].file_path, "test.txt");

    let search_by_filename_result = searcher.search(&super::SearchQuery {
        project_id: repository.project.id.clone(),
        q: "test.txt".to_string(),
        limit: 10,
        offset: None,
        range: Range { start: 0, end: 10 },
    });
    assert!(search_by_filename_result.is_ok());
    let search_by_filename_result = search_by_filename_result.unwrap();
    assert_eq!(search_by_filename_result.len(), 2);
    assert_eq!(search_by_filename_result[0].session_id, session.id);
    assert_eq!(
        search_by_filename_result[0].project_id,
        repository.project.id
    );
    assert_eq!(search_by_filename_result[0].file_path, "test.txt");

    let not_found_result = searcher.search(&super::SearchQuery {
        project_id: "not found".to_string(),
        q: "test.txt".to_string(),
        limit: 10,
        offset: None,
        range: Range { start: 0, end: 10 },
    });
    assert!(not_found_result.is_ok());
    let not_found_result = not_found_result.unwrap();
    assert_eq!(not_found_result.len(), 0);
}
