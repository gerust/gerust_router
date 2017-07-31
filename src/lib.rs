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
    AccessDenied,
    Subtree,
    NotFound,
    WithId(u64)
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

trait Pattern {
    fn match_recognizer(&self, recognizer: &mut Recognizer) -> bool;
}

impl<'a> Pattern for &'a str {
    fn match_recognizer(&self, recognizer: &mut Recognizer) -> bool {
        if recognizer.unmatched_path.starts_with(recognizer.seperator) {
            recognizer.unmatched_path = &recognizer.unmatched_path[1..];
        }

        if recognizer.unmatched_path.starts_with(self) {
            let (_, rest) = recognizer.unmatched_path.split_at(self.len());
            recognizer.unmatched_path = rest;
            true
        } else {
            false
        }
    }
}

impl<'a> Pattern for (&'a str, Method) {
    fn match_recognizer(&self, recognizer: &mut Recognizer) -> bool {
        if self.1 != recognizer.request.method {
            return false;
        }

        if recognizer.unmatched_path.starts_with(recognizer.seperator) {
            recognizer.unmatched_path = &recognizer.unmatched_path[1..];
        }

        if recognizer.unmatched_path.starts_with(self.0) {
            let (_, rest) = recognizer.unmatched_path.split_at(self.0.len());
            recognizer.unmatched_path = rest;
            true
        } else {
            false
        }
    }
}

impl<'a> Recognizer<'a> {
    fn root<F: Fn() -> Recognition>(&self, f: F) -> Result<(), Recognition> {
        if self.unmatched_path == "/" && self.request.method == Method::Get {
            Err(f())
        } else {
            Ok(())
        }
    }

    fn on<P: Pattern, F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, pattern: P, recognizer_fun: F) -> Result<(), Recognition> {
        if pattern.match_recognizer(self) {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn get<F: Fn(&mut Recognizer) -> Recognition>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Get {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn post<F: Fn(&mut Recognizer) -> Recognition>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Post {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn put<F: Fn(&mut Recognizer) -> Recognition>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Post {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn delete<F: Fn(&mut Recognizer) -> Recognition>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Delete {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn patch<F: Fn(&mut Recognizer) -> Recognition>(&mut self, recognizer_fun: F) -> Result<(), Recognition> {
        if self.request.method == Method::Patch {
            Err(recognizer_fun(self))
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

    fn mount<F: Fn(&mut Recognizer) -> Result<(), Recognition>>(&mut self, subtree: &RoutingTree<F>) -> Result<(), Recognition> {
        match subtree.traverse_with(self) {
            Ok(recognition) => Err(recognition),
            Err(()) => Ok(())
        }
    }

    fn param<F: std::str::FromStr>(&mut self) -> Result<F, Recognition> {
        if self.unmatched_path.starts_with(self.seperator) {
            self.unmatched_path = &self.unmatched_path[1..];
        }

        let maybe_loc = self.unmatched_path.find("/");

        let loc = match maybe_loc {
            Some(l) => l,
            None => return Err(Recognition::NotFound)
        };

        let (param, rest) = self.unmatched_path.split_at(loc);

        self.unmatched_path = rest;

        param.parse().map_err(|e| Recognition::NotFound )
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

        self.traverse_with(&mut rec)
    }

    fn traverse_with(&self, rec: &mut Recognizer) -> Result<Recognition, ()> {
        match (self.fun)(rec) {
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
                Recognition::Foo
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
fn test_path_verb_pairs() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on(("foo", Method::Get), |r| {
            r.get(|r| {
                Recognition::Foo
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
fn test_subroot() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            r.root(|| {
                Recognition::Foo
            })?;

            Ok(())
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo/").unwrap(),
    };
    let res = tree.recognize(&req);
    assert_eq!(res, Ok(Recognition::Foo));
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
                Recognition::Foo
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

#[test]
fn tree_in_tree() {
    let sub_tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Subtree
        })
    });
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            r.mount(&sub_tree)?;

            Ok(())
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo/").unwrap(),
    };
    let res = tree.recognize(&req);
    assert_eq!(res, Ok(Recognition::Subtree));
}


#[test]
fn test_params() {
    let tree = RoutingTree::route(|r| {
        r.root(|| {
            Recognition::Root
        })?;

        r.on("foo", |r| {
            let id = r.param()?;

            r.get(|r| {
                Recognition::WithId(id)
            })?;

            Ok(())
        })?;

        Ok(())
    });

    let req = MockRequest {
        method: Method::Get,
        url: Url::parse("http://localhost:9200/foo/1/").unwrap(),
    };
    let res = tree.recognize(&req);
    assert_eq!(res, Ok(Recognition::WithId(1)));
}