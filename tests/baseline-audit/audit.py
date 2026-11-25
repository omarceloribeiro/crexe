import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

BIN = Path(sys.argv[1])
ORIGINAL = Path(sys.argv[2])
RESULTS = []
REQUESTS = []
RESPONSE_FILES = []
HTTP_STATUS = 200
BASE = Path(tempfile.mkdtemp(prefix='crexe-baseline-', dir='/work'))
C_CODE = '#include <stdio.h>\nint main(void) { puts("CREXE_QA_APP_OK"); return 0; }\n'
STANDARD_FILES = [{'path': 'src/main.c', 'content': C_CODE}]


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        REQUESTS.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
        if HTTP_STATUS == 200:
            data = {'choices': [{'message': {'content': json.dumps({'files': RESPONSE_FILES})}}]}
        else:
            data = {'error': {'message': 'Synthetic provider error'}}
        encoded = json.dumps(data).encode()
        self.send_response(HTTP_STATUS)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def log_message(self, *args):
        pass


SERVER = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=SERVER.serve_forever, daemon=True).start()
PORT = SERVER.server_address[1]


def reset_provider(files=None, status=200):
    global RESPONSE_FILES, HTTP_STATUS
    RESPONSE_FILES = copy.deepcopy(STANDARD_FILES if files is None else files)
    HTTP_STATUS = status


def case(name):
    directory = BASE / name
    directory.mkdir()
    recipe = directory / 'application.crexe'
    spec = {
        'version': 1.0,
        'inputs': {'Theme': {'type': 'string', 'default': 'normal'}},
        'workspace': {'cache': {'root': {'linux': str(directory / 'cache')}, 'layout': 'workspace'}},
        'policies': {'timeoutSeconds': {'generate': 3, 'build': 1, 'run': 1},
                     'commandAllowlist': {'linux': ['mkdir', 'cc', 'sh', 'python3']}},
        'selectors': {'target': {'rules': [{'when': "{{OS}} == 'linux'", 'use': 'linux'}]}},
        'prompt': {'system': 'QA fixture', 'user_by_target': {'linux': 'Theme={{Theme}}'}},
        'generator': {'provider': 'openai-compatible', 'baseUrl': f'http://127.0.0.1:{PORT}/v1',
                      'apiKeyEnv': 'CREXE_QA_API_KEY', 'model': 'qa-mock', 'maxOutputTokens': 50},
        'targets': {'linux': {'build': {'steps': [{'cmd': ['mkdir', '-p', 'build']},
                                                {'cmd': ['cc', 'src/main.c', '-o', 'build/qa-app']}]},
                              'run': {'cmd': ['./build/qa-app']}}},
    }
    return directory, recipe, spec, directory / 'cache/workspace'


def save(recipe, spec):
    recipe.write_text(json.dumps(spec, ensure_ascii=False), encoding='utf-8')


def invoke(recipe=None, flags=(), direct=False, key=True, extra_env=None):
    args = [str(BIN)]
    if recipe is not None:
        if not direct:
            args.append('exec')
        args.append(str(recipe))
    args.extend(flags)
    env = os.environ.copy()
    env.pop('OPENAI_API_KEY', None)
    env.pop('CREXE_QA_API_KEY', None)
    if key:
        env['CREXE_QA_API_KEY'] = 'dummy-not-a-real-key'
    if extra_env:
        env.update(extra_env)
    before = len(REQUESTS)
    started = time.monotonic()
    result = subprocess.run(args, cwd='/tmp', env=env, text=True, capture_output=True, timeout=12)
    return {'exit': result.returncode, 'output': result.stdout + result.stderr,
            'requests': len(REQUESTS) - before, 'seconds': round(time.monotonic() - started, 3)}


def record(name, expected, passed, evidence):
    item = {'name': name, 'expected': expected, 'status': 'PASS' if passed else 'FAIL', 'evidence': evidence}
    RESULTS.append(item)
    print(f'{item["status"]}: {name}', flush=True)


reset_provider()
r = invoke(flags=['--help'])
record('cli_help', 'Help succeeds', r['exit'] == 0 and 'exec' in r['output'], r)
r = invoke(flags=['--version'])
record('cli_version', 'Version is available for support diagnostics', r['exit'] == 0, r)
r = invoke(ORIGINAL, key=False)
record('original_yaml_parsing', 'Original YAML reaches generation and reports missing key without a request',
       r['exit'] != 0 and 'Missing API key env var' in r['output'] and r['requests'] == 0, r)

d, p, s, w = case('happy path with spaces and accents ç')
save(p, s)
r = invoke(p)
record('generate_compile_run', 'Mock generation compiles real C and executes from a different CWD',
       r['exit'] == 0 and r['requests'] == 1 and 'CREXE_QA_APP_OK' in r['output'], r)
r = invoke(p, key=False)
record('cache_hit_without_api_key', 'Valid cache runs without generation or API key',
       r['exit'] == 0 and r['requests'] == 0 and 'Cache hit' in r['output'], r)
r = invoke(p, direct=True, key=False)
record('direct_invocation', 'File as first argument uses valid cache',
       r['exit'] == 0 and r['requests'] == 0, r)
r = invoke(p, flags=['--set', 'Theme=green'])
record('input_override', 'Changed input regenerates and appears in request prompt',
       r['exit'] == 0 and r['requests'] == 1 and 'Theme=green' in REQUESTS[-1]['messages'][1]['content'], r)
s['prompt']['user_by_target']['linux'] += ' Background green.'
save(p, s)
r = invoke(p)
record('edited_prompt_regenerates', 'Editing recipe invalidates cache', r['exit'] == 0 and r['requests'] == 1, r)
r = invoke(p, flags=['--rebuild'])
record('forced_rebuild', 'Rebuild forces generation', r['exit'] == 0 and r['requests'] == 1, r)
r = invoke(p, flags=['--set', 'bad-input'])
record('invalid_override', 'Invalid override fails without generation', r['exit'] != 0 and r['requests'] == 0, r)

d, p, s, w = case('bad-yaml')
p.write_text('version: [\n', encoding='utf-8')
r = invoke(p)
record('invalid_yaml', 'Invalid syntax fails before generation', r['exit'] != 0 and r['requests'] == 0, r)
d, p, s, w = case('future-version')
s['version'] = 999
save(p, s)
r = invoke(p)
record('unknown_spec_version', 'Unknown version fails before generation', r['exit'] != 0 and r['requests'] == 0, r)
d, p, s, w = case('missing-tool')
s['targets']['linux']['build']['steps'] = [{'cmd': ['crexe-qa-nonexistent-tool']}]
save(p, s)
r = invoke(p)
record('missing_tool_preflight', 'Invalid build requirement is detected before paid generation',
       r['exit'] != 0 and r['requests'] == 0, r)

d, p, s, w = case('empty-output')
save(p, s)
reset_provider([])
r = invoke(p)
record('empty_generator_files', 'Empty file list is rejected', r['exit'] != 0 and 'empty files' in r['output'], r)
d, p, s, w = case('provider-error')
save(p, s)
reset_provider(status=401)
r = invoke(p)
record('provider_http_error', 'HTTP error is reported without successful cache',
       r['exit'] != 0 and '401' in r['output'] and not (w / '.build_ok').exists(), r)

d, p, s, w = case('path-traversal')
save(p, s)
reset_provider([{'path': '../outside.txt', 'content': 'fixture'}])
r = invoke(p)
record('reject_parent_traversal', 'Parent traversal is rejected',
       r['exit'] != 0 and not (w.parent / 'outside.txt').exists(), r)
d, p, s, w = case('absolute-inside')
save(p, s)
reset_provider([{'path': str(w / 'src/main.c'), 'content': C_CODE}])
r = invoke(p)
record('reject_absolute_generated_path', 'Absolute output path is rejected as required by draft RFC',
       r['exit'] != 0 and not (w / 'src/main.c').exists(), r)
d, p, s, w = case('absolute-outside')
outside = d / 'outside/sub/file.txt'
save(p, s)
reset_provider([{'path': str(outside), 'content': 'fixture'}])
r = invoke(p)
r['outside_parent_created'] = outside.parent.exists()
record('no_external_directory_side_effect', 'Rejected external path creates no outside directories',
       r['exit'] != 0 and not outside.parent.exists(), r)
d, p, s, w = case('final-file-symlink')
outside = d / 'outside.txt'
outside.write_text('original sentinel')
(w / 'src').mkdir(parents=True)
(w / 'src/main.c').symlink_to(outside)
save(p, s)
reset_provider()
r = invoke(p)
r['outside_sentinel_unchanged'] = outside.read_text() == 'original sentinel'
record('reject_final_symlink_escape', 'Writing generated files cannot overwrite an external symlink target',
       r['exit'] != 0 and r['outside_sentinel_unchanged'], r)

d, p, s, w = case('empty-allowlist')
s['policies']['commandAllowlist']['linux'] = []
save(p, s)
r = invoke(p)
record('empty_allowlist_denies_commands', 'Empty command allowlist does not grant unrestricted execution', r['exit'] != 0, r)
d, p, s, w = case('build-timeout')
s['targets']['linux']['build']['steps'].insert(0, {'cmd': ['python3', '-c', 'import time;time.sleep(2.2)']})
save(p, s)
r = invoke(p)
record('build_timeout_enforced', 'A one-second timeout stops a command that waits over two seconds',
       r['exit'] != 0 and r['seconds'] < 2.0, r)
d, p, s, w = case('child-secret')
s['targets']['linux']['run']['cmd'] = ['/usr/bin/python3', '-c', 'import os;print(os.getenv("CREXE_QA_SECRET","ABSENT"))']
save(p, s)
r = invoke(p, extra_env={'CREXE_QA_SECRET': 'synthetic-secret-marker'})
record('child_environment_filtered', 'Build/run do not inherit an unrelated synthetic secret',
       r['exit'] == 0 and 'ABSENT' in r['output'] and 'synthetic-secret-marker' not in r['output'], r)

d, p, s, w = case('generated-run-outside')
outside = w.parent / 'outside.sh'
outside.parent.mkdir(parents=True)
outside.write_text('#!/bin/sh\necho CREXE_QA_OUTSIDE_WORKSPACE\n')
outside.chmod(0o755)
s['targets']['linux']['run']['cmd'] = ['./build/../../outside.sh']
save(p, s)
r = invoke(p)
record('generated_run_cannot_escape_workspace', 'A build prefix containing traversal does not authorize an outside executable',
       r['exit'] != 0 and 'CREXE_QA_OUTSIDE_WORKSPACE' not in r['output'], r)

d, p, s, w = case('failed-rebuild')
save(p, s)
reset_provider()
first = invoke(p)
reset_provider([{'path': 'src/main.c', 'content': 'this deliberately does not compile\n'}])
failed = invoke(p, flags=['--rebuild'])
reset_provider()
r = invoke(p)
record('failed_rebuild_not_silent_cache_hit', 'Failed replacement build is not silently treated as a valid new build',
       first['exit'] == 0 and failed['exit'] != 0 and 'Cache hit' not in r['output'],
       {'failed_rebuild': failed, 'next_run': r})

d, p, s, w = case('missing-artifact')
save(p, s)
invoke(p)
(w / 'build/qa-app').unlink()
r = invoke(p)
record('missing_cached_artifact_recovers', 'Missing artifact invalidates cache and rebuilds',
       r['exit'] == 0 and r['requests'] == 1, r)
d, p, s, w = case('corrupt-manifest')
save(p, s)
invoke(p)
(w / 'crexe.manifest.json').write_text('{broken')
r = invoke(p)
record('corrupt_manifest_recovers', 'Corrupt cache manifest is handled as an invalid cache entry',
       r['exit'] == 0 and r['requests'] == 1, r)
d, p, s, w = case('changed-artifact')
save(p, s)
invoke(p)
shutil.copyfile('/bin/true', w / 'build/qa-app')
r = invoke(p)
record('changed_cached_artifact_detected', 'Changed artifact is detected before cache reuse',
       r['requests'] == 1 and 'CREXE_QA_APP_OK' in r['output'], r)

SERVER.shutdown()
summary = {'scope': 'Linux container; original Rust source; synthetic local provider; no OpenAI calls',
           'pass': sum(item['status'] == 'PASS' for item in RESULTS),
           'fail': sum(item['status'] == 'FAIL' for item in RESULTS),
           'total': len(RESULTS), 'results': RESULTS}
Path('/qa/results.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps({key: value for key, value in summary.items() if key != 'results'}), flush=True)
sys.exit(1 if summary['fail'] else 0)
