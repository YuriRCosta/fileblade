.pragma library

var QUERIES = [
  { "query": "readme", "terms": [{ "text": "readme", "negate": false, "exact": false, "field": "" }], "filters": [] },
  { "query": "\"exact name\"", "terms": [{ "text": "exact name", "negate": false, "exact": true, "field": "" }], "filters": [] },
  { "query": "-draft notes", "terms": [{ "text": "draft", "negate": true, "exact": false, "field": "" }, { "text": "notes", "negate": false, "exact": false, "field": "" }], "filters": [] },
  { "query": "!draft", "terms": [{ "text": "draft", "negate": true, "exact": false, "field": "" }], "filters": [] },
  { "query": "name:plan", "terms": [{ "text": "plan", "negate": false, "exact": false, "field": "name" }], "filters": [] },
  { "query": "type:folder", "terms": [], "filters": [{ "key": "type", "value": "dir", "negate": false }] },
  { "query": "type:file,link", "terms": [], "filters": [{ "key": "type", "value": "file", "negate": false }, { "key": "type", "value": "link", "negate": false }] },
  { "query": "-type:image", "terms": [], "filters": [{ "key": "type", "value": "image", "negate": true }] },
  { "query": "invoice type:file", "terms": [{ "text": "invoice", "negate": false, "exact": false, "field": "" }], "filters": [{ "key": "type", "value": "file", "negate": false }] },
  { "query": "   ", "terms": [], "filters": [] },
  { "query": "'sub", "terms": [{ "text": "sub", "negate": false, "exact": false, "field": "" }], "filters": [] },
  { "query": "^pre", "terms": [{ "text": "pre", "negate": false, "exact": false, "field": "" }], "filters": [] },
  { "query": "suf$", "terms": [{ "text": "suf", "negate": false, "exact": false, "field": "" }], "filters": [] }
]
