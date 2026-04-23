window.BENCHMARK_DATA = {
  "lastUpdate": 1776972656528,
  "repoUrl": "https://github.com/simonmarty/aws-secretsmanager-agent",
  "entries": {
    "Secrets Manager Agent Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "martysi@amazon.com",
            "name": "Simon Marty",
            "username": "simonmarty"
          },
          "committer": {
            "email": "martysi@amazon.com",
            "name": "Simon Marty",
            "username": "simonmarty"
          },
          "distinct": true,
          "id": "9c5ae2d4480f2b093556d69dd13346695e7fce78",
          "message": "Squashed commit of the following:\n\ncommit 66edb868f1199515999a3df4ea2300f6039272c2\nMerge: 2a9b603 4b1852d\nAuthor: Simon Marty <martysi@amazon.com>\nDate:   Wed Apr 22 15:01:23 2026 -0700\n\n    Merge branch 'main' into bench-baseline\n\ncommit 2a9b6032030f52f15df93de11bb3332f82327957\nAuthor: Simon Marty <martysi@amazon.com>\nDate:   Mon Apr 20 11:41:02 2026 -0700\n\n    Update bench_baseline.yml\n\n    Signed-off-by: Simon Marty <martysi@amazon.com>\n\ncommit df1938d5410d63c94655d54b1c8e49dcfa718d6c\nAuthor: Simon Marty <martysi@amazon.com>\nDate:   Fri Apr 17 17:11:13 2026 -0700\n\n    Rename workflow\n\n    Signed-off-by: Simon Marty <martysi@amazon.com>\n\ncommit 7cfdbad0e5edc544ae181b658a4c661023c35ef6\nAuthor: Simon Marty <martysi@amazon.com>\nDate:   Fri Apr 17 16:37:10 2026 -0700\n\n    Push benchmark results to GitHub pages",
          "timestamp": "2026-04-22T16:41:29-07:00",
          "tree_id": "cce2077e6069b7524660c2f259cce4fb97f03a8f",
          "url": "https://github.com/simonmarty/aws-secretsmanager-agent/commit/9c5ae2d4480f2b093556d69dd13346695e7fce78"
        },
        "date": 1776972655661,
        "tool": "cargo",
        "benches": [
          {
            "name": "CacheHit",
            "value": 203,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "CacheEviction",
            "value": 70753,
            "range": "± 5962",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}