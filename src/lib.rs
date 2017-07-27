extern crate url;
extern crate futures;

use url::Url;

#[derive(Eq,PartialEq,Debug)]
enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch
}

#[derive(Eq,PartialEq,Debug)]
enum Recognition {
    Root,
    Foo,
    Bar,
    AccessDenied
}

#[derive(Debug)]
enum Error {
    NotFound
}

#[derive(Debug)]
struct MockRequest {
    method: Method,
    url: Url,
}

#[derive(Debug)]
struct Recognizer<'a> {
    request: &'a MockRequest,
    unmatched_path: &'a str,
    seperator: &'static str,
}

impl<'a> Recognizer<'a> {
    fn root<F: Fn() -> Recognition>(&self, f: F) -> Result<(), Recognition> {
        if self.unmatched_path == "/" && self.request.method == Method::Get {
            Err(f())
        } else {
            Ok(())
        }
    }

    fn on<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, path: &'static str, recognizer_fun: F) -> Result<(), Recognition> {
        if self.unmatched_path.starts_with(self.seperator) {
            self.unmatched_path = &self.unmatched_path[1..];
        }

        if self.unmatched_path.starts_with(path) {
            let (_, rest) = self.unmatched_path.split_at(path.len());
            self.unmatched_path = rest;
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn get<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Get {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn post<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Post {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn put<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Post {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn delete<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Delete {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn patch<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Patch {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn condition<F: Fn(&Recognizer) -> bool>(&mut self, predicate: F) -> Result<(), Recognition> {
        if predicate(self) {
            Ok(())
        } else {
            Err(Recognition::AccessDenied)
        }
    }
}

struct RoutingTree<F: Fn(&mut Recognizer) -> Result<(), Recognition>> {
    fun: F
}

impl<F: Fn(&mut Recognizer) -> Result<(),Recognition>> RoutingTree<F> {
    fn route(route_fn: F) -> RoutingTree<F> {
        RoutingTree { fun: route_fn }
    }

    fn recognize(&self, request: &MockRequest) -> Result<Recognition, ()> {
        let mut rec = Recognizer { request: request, unmatched_path: request.url.path(), seperator: "/"};

        match (self.fun)(&mut rec) {
            Ok(()) => Err(()),
            Err(recognition) => Ok(recognition)
        }
    }
}

#[test]
fn test() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200").unwrap(),
    };
    assert!(tree.recognize(&req).is_ok());
    assert!(tree.recognize(&req).is_ok());
}

#[test]
fn test_path() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |_| {
            Err(Recognition::Foo)
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo").unwrap(),
    };
    assert!(tree.recognize(&req).is_ok());
    assert!(tree.recognize(&req).is_ok());
}

#[test]
fn test_sub_path() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            r.on("bar", |r| {
                Err(Recognition::Bar)
            })?;

            Err(Recognition::Foo)
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo/bar").unwrap(),
    };
    assert!(tree.recognize(&req).is_ok());
    assert!(tree.recognize(&req).is_ok());
}

#[test]
fn test_verbs() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            r.get(|r| {
                Err(Recognition::Foo)
            })?;

            Ok(())
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo").unwrap(),
    };
    assert!(tree.recognize(&req).is_ok());

    let req = MockRequest {
        method: Method::Post,
        url: Url::parse("http://localhost:9200/foo").unwrap(),
    };
    assert!(tree.recognize(&req).is_err());
}

#[test]
fn test_conditions() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            r.condition(|r| {
                r.request.method == Method::Post
            })?;

            r.get(|r| {
                Err(Recognition::Foo)
            })?;

            Ok(())
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo").unwrap(),
    };
    let res = tree.recognize(&req);
    assert_eq!(res, Ok(Recognition::AccessDenied));
}