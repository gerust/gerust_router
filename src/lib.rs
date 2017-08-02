extern crate url;
extern crate http;

use http::{method, Method};

pub trait RouteResult {
    fn access_denied() -> Self;
    fn not_found() -> Self;
}

pub trait HttpRequest {
    fn method(&self) -> Method;
    fn path(&self) -> &str;
}

#[derive(Debug)]
pub struct Recognizer<'a, R: HttpRequest + 'a> {
    request: &'a R,
    unmatched_path: &'a str,
    seperator: &'static str,
}

pub trait Pattern {
    fn match_recognizer<R: HttpRequest>(&self, recognizer: &mut Recognizer<R>) -> bool;
}

impl<'a> Pattern for &'a str {
    fn match_recognizer<R: HttpRequest>(&self, recognizer: &mut Recognizer<R>) -> bool {
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
    fn match_recognizer<R: HttpRequest>(&self, recognizer: &mut Recognizer<R>) -> bool {
        if self.1 != recognizer.request.method() {
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

pub trait Recognize<R: RouteResult> {
    fn root<F: Fn() -> R>(&self, f: F) -> Result<(), R>;
    fn on<P: Pattern, F: Fn(&mut Self) -> Result<(), R>>(&mut self, pattern: P, recognizer_fun: F) -> Result<(), R>;

    fn get<F: Fn(&mut Self) -> R>(&mut self, recognizer_fun: F) -> Result<(), R>;

    fn post<F: Fn(&mut Self) -> R>(&mut self, recognizer_fun: F) -> Result<(), R>;

    fn put<F: Fn(&mut Self) -> R>(&mut self, recognizer_fun: F) -> Result<(), R>;

    fn delete<F: Fn(&mut Self) -> R>(&mut self, recognizer_fun: F) -> Result<(), R>;

    fn patch<F: Fn(&mut Self) -> R>(&mut self, recognizer_fun: F) -> Result<(), R>;

    fn param<F: std::str::FromStr>(&mut self, name: &'static str) -> Result<Param<F>, R>;
}

pub trait Mount<Req: HttpRequest, Rec: RouteResult> {
    fn mount<T: RoutingTreeTrait<Req, Rec>>(&mut self, subtree: &T) -> Result<(), Rec>;
}

pub trait Condition<Req: HttpRequest, Rec: RouteResult> {
    fn condition<F: Fn(&Self) -> bool>(&mut self, predicate: F) -> Result<(), Rec>;
}

impl<'a, Req: HttpRequest, Rec: RouteResult> Recognize<Rec> for Recognizer<'a, Req> {
    fn root<F: Fn() -> Rec>(&self, f: F) -> Result<(), Rec> {
        if self.unmatched_path == "/" && self.request.method() == method::GET {
            Err(f())
        } else {
            Ok(())
        }
    }

    fn on<P: Pattern, F: Fn(&mut Self) -> Result<(), Rec>>(&mut self, pattern: P, recognizer_fun: F) -> Result<(), Rec> {
        if pattern.match_recognizer(self) {
            recognizer_fun(self)
        } else {
            Ok(())
        }
    }

    fn get<F: Fn(&mut Self) -> Rec>(&mut self, recognizer_fun: F) -> Result<(), Rec> {
        if self.request.method() == method::GET {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn post<F: Fn(&mut Self) -> Rec>(&mut self, recognizer_fun: F) -> Result<(), Rec> {
        if self.request.method() == method::POST {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn put<F: Fn(&mut Self) -> Rec>(&mut self, recognizer_fun: F) -> Result<(), Rec> {
        if self.request.method() == method::PUT {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn delete<F: Fn(&mut Self) -> Rec>(&mut self, recognizer_fun: F) -> Result<(), Rec> {
        if self.request.method() == method::DELETE {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn patch<F: Fn(&mut Self) -> Rec>(&mut self, recognizer_fun: F) -> Result<(), Rec> {
        if self.request.method() == method::PATCH {
            Err(recognizer_fun(self))
        } else {
            Ok(())
        }
    }

    fn param<F: std::str::FromStr>(&mut self, name: &'static str) -> Result<Param<F>, Rec> {
        if self.unmatched_path.starts_with(self.seperator) {
            self.unmatched_path = &self.unmatched_path[1..];
        }

        let maybe_loc = self.unmatched_path.find("/");

        let loc = match maybe_loc {
            Some(l) => l,
            None => return Err(RouteResult::not_found())
        };

        let (param, rest) = self.unmatched_path.split_at(loc);

        self.unmatched_path = rest;

        param.parse().map(|p| Param { val: p, name: name }).map_err(|e| RouteResult::not_found() )
    }
}

impl<'a, Req: HttpRequest, Rec: RouteResult> Mount<Req, Rec> for Recognizer<'a, Req> {
    fn mount<Tree: RoutingTreeTrait<Req, Rec>>(&mut self, subtree: &Tree) -> Result<(), Rec> {
        match subtree.traverse_with(self) {
            Ok(recognition) => Err(recognition),
            Err(()) => Ok(())
        }
    }
}

impl<'a, Req: HttpRequest, Rec: RouteResult> Condition<Req, Rec> for Recognizer<'a, Req> {
    fn condition<F: Fn(&Self) -> bool>(&mut self, predicate: F) -> Result<(), Rec> {
        if predicate(self) {
            Ok(())
        } else {
            Err(RouteResult::access_denied())
        }
    }
}

pub struct Param<T> {
    val: T,
    name: &'static str
}

impl<T> std::ops::Deref for Param<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.val
    }
}

pub struct RoutingTree<F> {
    fun: F,
}

impl<F> RoutingTree<F> {
    pub fn route<Req, Res>(route_fn: F) -> Self where F: Fn(&mut Recognizer<Req>) -> Result<(),Res> {
        RoutingTree { fun : route_fn }
    }
}

pub trait RoutingTreeTrait<Req: HttpRequest, Res: RouteResult> {
    fn recognize(&self, request: &Req) -> Result<Res, ()>;

    fn traverse_with(&self, rec: &mut Recognizer<Req>) -> Result<Res, ()>;
}

impl<Req: HttpRequest, Res: RouteResult, F: Fn(&mut Recognizer<Req>) -> Result<(),Res>> RoutingTreeTrait<Req, Res> for RoutingTree<F>  {
    fn recognize(&self, request: &Req) -> Result<Res, ()> {
        let mut rec = Recognizer { request: request, unmatched_path: request.path(), seperator: "/"};

        self.traverse_with(&mut rec)
    }

    fn traverse_with(&self, rec: &mut Recognizer<Req>) -> Result<Res, ()> where F: Fn(&mut Recognizer<Req>) -> Result<(),Res> {
        match (self.fun)(rec) {
            Ok(()) => Err(()),
            Err(recognition) => Ok(recognition)
        }
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use http::{method, Method};
    use super::RoutingTree;
    use super::HttpRequest;
    use super::Recognize;
    use super::RouteResult;
    use super::RoutingTreeTrait;
    use super::Mount;

    #[derive(Debug)]
    struct MockRequest {
        method: Method,
        url: Url,
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


    impl RouteResult for Recognition {
        fn access_denied() -> Self {
            Recognition::AccessDenied
        }

        fn not_found() -> Self {
            Recognition::NotFound
        }
    }


    impl HttpRequest for MockRequest {
        fn method(&self) -> Method {
            self.method.clone()
        }

        fn path(&self) -> &str {
            self.url.path()
        }
    }

    #[test]
    fn test() {
        let tree = RoutingTree::route(|r| {
            r.root(|| {
                Recognition::Root
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200").unwrap(),
        };
        assert!(tree.recognize(&req).is_ok());
        assert!(tree.recognize(&req).is_ok());
    }

    #[test]
    fn test_path() {
        let tree = RoutingTree::route::<MockRequest, Recognition>(|r| {
            r.root(|| {
                Recognition::Root
            })?;

            r.on("foo", |_| {
                Err(Recognition::Foo)
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo").unwrap(),
        };
        assert!(tree.recognize(&req).is_ok());
        assert!(tree.recognize(&req).is_ok());
    }

    #[test]
    fn test_sub_path() {
        let tree = RoutingTree::route::<MockRequest, Recognition>(|r| {
            r.root(|| {
                Recognition::Root
            })?;

            r.on("foo", |r| {
                r.on("bar", |r| {
                    Err(Recognition::Bar)
                })?;

                Err(Recognition::Foo)
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo/bar").unwrap(),
        };
        assert!(tree.recognize(&req).is_ok());
        assert!(tree.recognize(&req).is_ok());
    }

    #[test]
    fn test_verbs() {
        let tree = RoutingTree::route::<MockRequest, Recognition>(|r| {
            r.root(|| {
                Recognition::Root
            })?;

            r.on("foo", |r| {
                r.get(|r| {
                    Recognition::Foo
                })
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo").unwrap(),
        };
        assert!(tree.recognize(&req).is_ok());

        let req = MockRequest {
            method: method::POST,
            url: Url::parse("http://localhost:9200/foo").unwrap(),
        };
        assert!(tree.recognize(&req).is_err());
    }

    #[test]
    fn test_path_verb_pairs() {
        let tree = RoutingTree::route::<MockRequest, Recognition>(|r| {
            r.root(|| {
                Recognition::Root
            })?;

            r.on(("foo", method::GET), |r| {
                r.get(|r| {
                    Recognition::Foo
                })
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo").unwrap(),
        };
        assert!(tree.recognize(&req).is_ok());

        let req = MockRequest {
            method: method::POST,
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
                })
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo/").unwrap(),
        };
        let res = tree.recognize(&req);
        assert_eq!(res, Ok(Recognition::Foo));
    }


//    #[test]
//    fn test_conditions() {
//        let tree = RoutingTree::route(|r| {
//            r.root(|| {
//                Recognition::Root
//            })?;
//
//            r.on("foo", |r| {
//                r.condition(|r| {
//                    &r.request().method() == method::POST
//                })?;
//
//                r.get(|r| {
//                    Recognition::Foo
//                })?;
//
//                Ok(())
//            })?;
//
//            Ok(())
//        });
//
//        let req = MockRequest {
//            method: method::GET,
//            url: Url::parse("http://localhost:9200/foo").unwrap(),
//        };
//        let res = tree.recognize(&req);
//        assert_eq!(res, Ok(Recognition::AccessDenied));
//    }

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
                r.mount(&sub_tree)
            })?;

            Ok(())
        });

        let req = MockRequest {
            method: method::GET,
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
                let id = r.param("id")?;

                r.get(|r| {
                    Recognition::WithId(*id)
                })
            })
        });

        let req = MockRequest {
            method: method::GET,
            url: Url::parse("http://localhost:9200/foo/1/").unwrap(),
        };
        let res = tree.recognize(&req);
        assert_eq!(res, Ok(Recognition::WithId(1)));
    }
}