// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use drawbridge_http::http::{self, Error, Request, Response, StatusCode};
use drawbridge_http::{async_trait, Handler};
use drawbridge_tags as tag;
use drawbridge_tree as tree;

#[derive(Clone, Default)]
pub struct Service {
    tree: tree::Service<tree::Memory>,
    tag: tag::Service<tag::Memory>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Namespace {
    owner: String,
    groups: Vec<String>,
    name: String,
}

impl FromStr for Namespace {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-'))
                    .is_none()
        }

        let mut namespace = s.split('/').map(Into::into);
        let owner = namespace
            .next()
            .ok_or("Repository owner must be specified")?;
        let mut namespace = namespace.collect::<Vec<_>>();
        let name = namespace.pop().ok_or("Repository name must be specified")?;
        let groups = namespace;
        if !valid(&owner) || !valid(&name) || !groups.iter().all(valid) {
            Err("Invalid namespace")
        } else {
            Ok(Self {
                owner,
                groups,
                name,
            })
        }
    }
}

#[async_trait]
impl Handler<()> for Service {
    type Response = http::Result<Response>;

    async fn handle(self, mut req: Request) -> Self::Response {
        fn no_route() -> Error {
            Error::from_str(StatusCode::NotFound, "Route not found")
        }

        let url = req.url_mut();
        let path = url.path();
        let (namespace, path) = path
            .strip_prefix('/')
            .expect("invalid URI")
            .split_once("/_")
            .ok_or_else(no_route)?;

        let namespace = namespace
            .parse()
            .map_err(|e| Error::from_str(StatusCode::BadRequest, e))?;

        let path = path.to_string();
        let (comp, path) = path.split_once('/').unwrap_or((&path, ""));
        url.set_path(&format!("/{}", path));

        // TODO: use `namespace`
        let _: Namespace = namespace;

        match comp {
            "tree" => Ok(self.tree.handle(req).await),
            "tag" => Ok(self.tag.handle(req).await),
            _ => Err(no_route()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_from_str() {
        assert!("".parse::<Namespace>().is_err());
        assert!(" ".parse::<Namespace>().is_err());
        assert!("/".parse::<Namespace>().is_err());
        assert!("name".parse::<Namespace>().is_err());
        assert!("owner/".parse::<Namespace>().is_err());
        assert!("/name".parse::<Namespace>().is_err());
        assert!("owner//name".parse::<Namespace>().is_err());
        assert!("owner/name/".parse::<Namespace>().is_err());
        assert!("owner/group///name".parse::<Namespace>().is_err());
        assert!("owner/g%roup/name".parse::<Namespace>().is_err());
        assert!("owner/gяoup/name".parse::<Namespace>().is_err());
        assert!("owner /group/name".parse::<Namespace>().is_err());
        assert!("owner/gr☣up/name".parse::<Namespace>().is_err());
        assert!("o.wner/group/name".parse::<Namespace>().is_err());

        assert_eq!(
            "owner/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec![],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec!["group".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/subgroup/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec!["group".into(), "subgroup".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "0WnEr/gr0up/subgr0up/-n4mE".parse(),
            Ok(Namespace {
                owner: "0WnEr".into(),
                groups: vec!["gr0up".into(), "subgr0up".into()],
                name: "-n4mE".into(),
            })
        );
    }
}
