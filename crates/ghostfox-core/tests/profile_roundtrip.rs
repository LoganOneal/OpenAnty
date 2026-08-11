use ghostfox_core::GhostfoxService;
use ghostfox_proto::CreateProfileRequest;
use tempfile::tempdir;

#[test]
fn create_list_delete_profile() {
    let dir = tempdir().unwrap();
    let (svc, _) = GhostfoxService::init(dir.path().to_path_buf()).unwrap();
    let p = svc
        .create_profile(CreateProfileRequest {
            name: "test".into(),
            template: Some("win11_chrome_mid".into()),
            os: None,
            proxy: None,
            fingerprint_overrides: None,
            tags: Some(vec!["ci".into()]),
            notes: None,
        })
        .unwrap();
    assert!(p.id.starts_with("prf_"));
    assert!(!p.fingerprint_hash.is_empty());
    let list = svc.list_profiles(10).unwrap();
    assert_eq!(list.len(), 1);
    svc.delete_profile(&p.id).unwrap();
    let list2 = svc.list_profiles(10).unwrap();
    assert!(list2.is_empty());
}

#[test]
fn cookie_import_export() {
    let dir = tempdir().unwrap();
    let (svc, _) = GhostfoxService::init(dir.path().to_path_buf()).unwrap();
    let p = svc
        .create_profile(CreateProfileRequest {
            name: "cookies".into(),
            template: None,
            os: None,
            proxy: None,
            fingerprint_overrides: None,
            tags: None,
            notes: None,
        })
        .unwrap();
    let cookies = vec![ghostfox_proto::Cookie {
        name: "sid".into(),
        value: "abc".into(),
        domain: ".example.com".into(),
        path: "/".into(),
        expires: -1.0,
        http_only: true,
        secure: true,
        same_site: Some("Lax".into()),
        partition_key: None,
    }];
    let (n, _, pending) = svc.import_cookies(&p.id, cookies, true).unwrap();
    assert_eq!(n, 1);
    assert!(pending);
    let out = svc.export_cookies(&p.id).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "sid");
}
