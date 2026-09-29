const GRAPHIQL_VERSION: &str = "3.9.0";

pub fn html(endpoint: &str) -> String {
    let endpoint_json =
        serde_json::to_string(endpoint).expect("GraphiQL endpoint should serialize as JSON");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="robots" content="noindex">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="referrer" content="origin">
    <title>App Framework GraphiQL</title>
    <style>
      body {{
        height: 100%;
        margin: 0;
        width: 100%;
        overflow: hidden;
      }}

      #graphiql {{
        height: 100vh;
      }}
    </style>
    <script crossorigin src="https://unpkg.com/react@18/umd/react.development.js"></script>
    <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
    <link rel="icon" href="https://graphql.org/favicon.ico">
    <link rel="stylesheet" href="https://unpkg.com/graphiql@{GRAPHIQL_VERSION}/graphiql.min.css" />
  </head>
  <body>
    <div id="graphiql">Loading...</div>
    <script src="https://unpkg.com/graphiql@{GRAPHIQL_VERSION}/graphiql.min.js" type="application/javascript"></script>
    <script>
      const customFetch = (url, opts = {{}}) => fetch(url, {{ ...opts, credentials: "same-origin" }});

      const createUrl = (endpoint) => new URL(endpoint, window.location.origin).toString();

      ReactDOM.createRoot(document.getElementById("graphiql")).render(
        React.createElement(GraphiQL, {{
          fetcher: GraphiQL.createFetcher({{
            url: createUrl({endpoint_json}),
            fetch: customFetch,
          }}),
          defaultEditorToolsVisibility: true,
        }})
      );
    </script>
  </body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_graphiql_assets_to_known_umd_build() {
        let source = html("/crm");

        assert!(source.contains("https://unpkg.com/graphiql@3.9.0/graphiql.min.css"));
        assert!(source.contains("https://unpkg.com/graphiql@3.9.0/graphiql.min.js"));
        assert!(source.contains("ReactDOM.createRoot"));
        assert!(source.contains(r#"url: createUrl("/crm")"#));
    }
}
