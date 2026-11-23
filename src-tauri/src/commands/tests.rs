use super::*;

#[test]
fn startup_input_accepts_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("photo.png");
    std::fs::write(&file, b"not really a png").unwrap();

    let resolved = resolve_startup_input(Some(file.into_os_string())).unwrap();
    assert!(resolved.ends_with("photo.png"));
}

#[test]
fn startup_input_ignores_missing_paths_and_directories() {
    assert!(resolve_startup_input(None).is_none());
    assert!(resolve_startup_input(Some(OsString::from("/does/not/exist.png"))).is_none());

    let dir = tempfile::tempdir().unwrap();
    assert!(resolve_startup_input(Some(dir.path().as_os_str().to_owned())).is_none());
}

#[test]
fn pending_open_is_taken_once() {
    let pending = PendingOpen::default();
    assert!(pending.take().is_none());

    pending.set("/tmp/photo.png".into());
    assert_eq!(pending.take().as_deref(), Some("/tmp/photo.png"));
    assert!(pending.take().is_none());
}

#[test]
fn output_exists_maps_to_an_overwrite_action() {
    let api = ApiError::from(TileError::OutputExists(PathBuf::from("/tmp/tiles.pdf")));
    assert_eq!(api.kind, ApiErrorKind::OutputExists);
    assert!(api.message.contains("tiles.pdf"));
}

#[test]
fn validation_errors_keep_their_kind() {
    assert_eq!(
        ApiError::from(TileError::InvalidTileSize).kind,
        ApiErrorKind::InvalidTileSize
    );
    assert_eq!(
        ApiError::from(TileError::InvalidDpi).kind,
        ApiErrorKind::InvalidDpi
    );
    assert_eq!(
        ApiError::from(TileError::InvalidOverlap).kind,
        ApiErrorKind::InvalidOverlap
    );
    assert_eq!(
        ApiError::from(TileError::InvalidMargin).kind,
        ApiErrorKind::InvalidMargin
    );
    assert_eq!(
        ApiError::from(TileError::PageTooSmall).kind,
        ApiErrorKind::PageTooSmall
    );
    assert_eq!(
        ApiError::from(TileError::InvalidInputName(PathBuf::from("x"))).kind,
        ApiErrorKind::InvalidInputName
    );
    assert_eq!(
        ApiError::from(TileError::Pdf("boom".into())).kind,
        ApiErrorKind::Pdf
    );
}

#[test]
fn kind_serializes_as_snake_case() {
    assert_eq!(
        serde_json::to_value(ApiErrorKind::OutputExists).unwrap(),
        serde_json::json!("output_exists")
    );
    assert_eq!(
        serde_json::to_value(ApiErrorKind::InvalidDpi).unwrap(),
        serde_json::json!("invalid_dpi")
    );
}
