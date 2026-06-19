use crate::model::RouteSegmentKind;

pub(crate) fn classify_route_segment(path: &str) -> RouteSegmentKind {
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    if lowered.contains("/tests/")
        || lowered.contains("/test/")
        || lowered.ends_with("_test.rs")
        || lowered.ends_with("_test.py")
        || lowered.ends_with("test.rs")
        || lowered.ends_with("test.py")
        || lowered
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
    {
        return RouteSegmentKind::Test;
    }
    if lowered.contains("/alembic/")
        || lowered.starts_with("alembic/")
        || lowered.contains("/migrations/")
        || lowered.starts_with("migrations/")
        || lowered.contains("/versions/")
        || lowered.starts_with("versions/")
    {
        return RouteSegmentKind::Migration;
    }
    if lowered.contains("validator") {
        return RouteSegmentKind::Service;
    }
    if lowered.contains("crud") {
        return RouteSegmentKind::Crud;
    }
    if lowered.contains("query") || lowered.ends_with(".sql") {
        return RouteSegmentKind::Query;
    }
    if lowered.contains("service") {
        return RouteSegmentKind::Service;
    }
    if lowered.contains("/lib/api/")
        || lowered.starts_with("lib/api/")
        || lowered.starts_with("frontend/src/lib/api/")
    {
        return RouteSegmentKind::ApiClient;
    }
    if lowered.contains("endpoint")
        || lowered.contains("controller")
        || lowered.contains("/api/")
        || lowered.contains("/routes/")
    {
        return RouteSegmentKind::Endpoint;
    }
    if lowered.contains("client") {
        return RouteSegmentKind::ApiClient;
    }
    if lowered.contains("hook")
        || lowered.contains("/frontend/")
        || lowered.starts_with("frontend/")
        || lowered.contains("/ui/")
        || lowered.starts_with("ui/")
    {
        return RouteSegmentKind::Ui;
    }
    RouteSegmentKind::Unknown
}

pub(crate) fn classify_route_source_kind(path: &str) -> &'static str {
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    if lowered.contains("/tests/")
        || lowered.contains("/test/")
        || lowered.ends_with("_test.rs")
        || lowered.ends_with("_test.py")
        || lowered.ends_with("test.rs")
        || lowered.ends_with("test.py")
        || lowered
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
    {
        return "test";
    }
    if lowered.contains("/alembic/")
        || lowered.starts_with("alembic/")
        || lowered.contains("/migrations/")
        || lowered.starts_with("migrations/")
        || lowered.contains("/versions/")
        || lowered.starts_with("versions/")
    {
        return "migration";
    }
    if lowered.contains("validator") {
        return "validator";
    }
    if lowered.contains("crud") {
        return "crud";
    }
    if lowered.contains("query") || lowered.ends_with(".sql") {
        return "query";
    }
    if lowered.contains("model") || lowered.contains("schema") {
        return "model";
    }
    if lowered.contains("service") {
        return "service";
    }
    if lowered.contains("/lib/api/")
        || lowered.starts_with("lib/api/")
        || lowered.starts_with("frontend/src/lib/api/")
    {
        return "api_client";
    }
    if lowered.contains("endpoint")
        || lowered.contains("controller")
        || lowered.contains("/api/")
        || lowered.contains("/routes/")
    {
        return "endpoint";
    }
    if lowered.contains("client") {
        return "api_client";
    }
    if lowered.contains("hook")
        || lowered.contains("/frontend/")
        || lowered.starts_with("frontend/")
        || lowered.contains("/ui/")
        || lowered.starts_with("ui/")
    {
        return "ui";
    }
    if lowered.contains("constraint") || lowered.contains("index") {
        return "constraint_source";
    }
    "unknown"
}

pub(crate) fn route_kind_label(kind: RouteSegmentKind) -> &'static str {
    match kind {
        RouteSegmentKind::Ui => "ui",
        RouteSegmentKind::ApiClient => "api_client",
        RouteSegmentKind::Endpoint => "endpoint",
        RouteSegmentKind::Service => "service",
        RouteSegmentKind::Crud => "crud",
        RouteSegmentKind::Query => "query",
        RouteSegmentKind::Test => "test",
        RouteSegmentKind::Migration => "migration",
        RouteSegmentKind::Unknown => "unknown",
    }
}
