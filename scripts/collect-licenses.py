#!/usr/bin/env python3
"""Collect exact-version upstream notices; review the resulting changes before release."""
import os, tempfile, concurrent.futures, hashlib, io, json, pathlib, re, shutil, subprocess, urllib.request, zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'licenses'
TEXTS = OUT / 'texts'
TEXTS.mkdir(parents=True, exist_ok=True)
CACHE = pathlib.Path(os.environ.get('NETMON_LICENSE_CACHE', str(pathlib.Path(tempfile.gettempdir()) / 'netmon-license-downloads')))
CACHE.mkdir(exist_ok=True)
META = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--manifest-path', str(ROOT / 'src-tauri/Cargo.toml')]))
PACKAGES = {p['name']:p for p in META['packages']}

def atomic_write(path, data):
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as f:
        f.write(data)
        temp=f.name
    os.replace(temp,path)

def get(url):
    f = CACHE / hashlib.sha256(url.encode()).hexdigest()
    if not f.exists():
        req = urllib.request.Request(url, headers={'User-Agent':'netmon-license-inventory', 'Accept':'application/vnd.github+json' if 'api.github.com' in url else '*/*'})
        with urllib.request.urlopen(req, timeout=60) as response:
            atomic_write(f, response.read())
    return f.read_bytes()

def doc(name, content, source):
    if isinstance(content, bytes): content = content.decode('utf-8')
    digest = hashlib.sha256(content.encode()).hexdigest()
    atomic_write(TEXTS / (digest+'.txt'), content.encode('utf-8'))
    return {'name':name, 'file':'texts/'+digest+'.txt', 'sha256':digest, 'source':source}

def legal_name(name):
    return bool(re.match(r'^(unlicense|licen[sc]e|copying|notice|copyright|authors)([._-].*)?$', name, re.I))

def local_docs(d, source):
    found=[]
    for f in sorted(d.rglob('*')):
        if not f.is_file() or '.git' in f.parts or 'node_modules' in f.relative_to(d).parts:continue
        rel=f.relative_to(d)
        if legal_name(f.name) or (any(x.lower() in ('licenses','licences') for x in rel.parts[:-1]) and f.suffix.lower() in ('.txt','.md','.html','')):
            if f.stat().st_size > 2_000_000:continue
            try: text=f.read_text(encoding='utf-8')
            except UnicodeError:continue
            if len(text)>30:found.append(doc(str(rel),text,source+' :: '+str(rel)))
    return found

def has_terms(docs):
    text='\n'.join((OUT/x['file']).read_text() for x in docs)
    return bool(re.search(r'Permission is hereby granted|Redistribution and use|Apache License|Mozilla Public License|DO WHAT THE FUCK|Permission to use, copy|This software is provided|CC0 1.0 Universal|This is free and unencumbered',text,re.I))

def upstream_docs(repo, sha, subpath=''):
    repo=repo.removesuffix('/').removesuffix('.git').removeprefix('git+').removeprefix('https://github.com/')
    tree=json.loads(get(f'https://api.github.com/repos/{repo}/git/trees/{sha}?recursive=1'))
    if tree.get('truncated'):raise ValueError('truncated tree '+repo)
    parents={'.',''}
    p=pathlib.PurePosixPath(subpath)
    parents.update(str(x) for x in (p,*p.parents))
    found=[]
    for entry in tree['tree']:
        path=pathlib.PurePosixPath(entry['path'])
        if entry['type']!='blob' or not legal_name(path.name) or str(path.parent) not in parents:continue
        if entry.get('size',0)<100:continue
        url=f'https://raw.githubusercontent.com/{repo}/{sha}/{path}'
        found.append(doc(str(path),get(url),url))
    return found

def collect_rust(p):
    if p['name']=='netmon':return None
    d=pathlib.Path(p['manifest_path']).parent
    source=p['source']
    url=(f'https://static.crates.io/crates/{p["name"]}/{p["name"]}-{p["version"]}.crate' if source.startswith('registry+') else source.removeprefix('git+'))
    docs=local_docs(d,url)
    if not has_terms(docs):
        info=json.loads((d/'.cargo_vcs_info.json').read_text()) if (d/'.cargo_vcs_info.json').exists() else {}
        # Published companion crates omit root license files; the referenced
        # companion source package has the same declared license and author.
        companions={'libappindicator-sys':'libappindicator','tao-macros':'tao','winapi-i686-pc-windows-gnu':'winapi','winapi-x86_64-pc-windows-gnu':'winapi'}
        if p['name'] in companions:
            q=PACKAGES[companions[p['name']]]
            assert set(re.findall(r'[A-Za-z0-9.-]+', p['license'])) == set(re.findall(r'[A-Za-z0-9.-]+', q['license'])) or (p['name'] == 'tao-macros' and p['license'] == 'MIT OR Apache-2.0' and q['license'] == 'Apache-2.0'), 'Companion license changed; review '+p['name']
            docs+=local_docs(pathlib.Path(q['manifest_path']).parent,f'https://static.crates.io/crates/{q["name"]}/{q["name"]}-{q["version"]}.crate')
        else:
            docs+=upstream_docs(p['repository'],info['git']['sha1'],info.get('path_in_vcs',''))
    if not has_terms(docs):
        if p['name'] not in ('selectors','sigchld') and not p['name'].startswith(('objc2','block2','dispatch2')):
            raise ValueError('missing full license '+p['name'])
        docs.append(doc('Published license declaration', 'License: '+p['license']+'\nAuthors (package metadata): '+', '.join(p.get('authors',[]))+'\nFull standard terms are included separately. Existing upstream notices are preserved above.\n', url+' :: Cargo.toml'))
    item={'ecosystem':'cargo','name':p['name'],'version':p['version'],'license':p['license'],'source':source,'download':url,'documents':docs}
    if p['name'] == 'tao-macros': item['selected_license']='Apache-2.0'
    if 'MPL-2.0' in p['license']:
        archive=next(pathlib.Path.home().glob('.cargo/registry/cache/*/'+p['name']+'-'+p['version']+'.crate'))
        dest=ROOT/'static'/'legal-sources'/archive.name
        dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(archive,dest)
        item['source_archive']={'file':str(dest.relative_to(ROOT)), 'sha256':hashlib.sha256(dest.read_bytes()).hexdigest()}
    return item

def collect_npm(key,p):
    name=key.split('node_modules/')[-1]
    d=ROOT/key
    companions={ '@esbuild/':'esbuild', '@rollup/rollup-':'rollup', '@tauri-apps/cli-':'@tauri-apps/cli' }
    companion=next((parent for prefix,parent in companions.items() if name.startswith(prefix)),None)
    if companion:
        other=LOCK['packages']['node_modules/'+companion]
        assert other['version']==p['version']
        assert p['license'] == other['license'], 'Platform package license changed; review '+name
        docs=local_docs(ROOT/'node_modules'/companion,other['resolved'])
        package={'repository':None}
    else:
        package=json.loads((d/'package.json').read_text())
        docs=local_docs(d,p['resolved'])
    if not has_terms(docs):
        meta=json.loads(get('https://registry.npmjs.org/'+name.replace('/','%2f')+'/'+p['version']))
        repo=meta.get('repository',package.get('repository'))
        if isinstance(repo,dict):repo=repo['url']
        repo=repo.removeprefix('git+').removesuffix('.git')
        if repo.startswith('git://github.com/'):repo=repo.replace('git://','https://')
        if not repo.startswith('http'):repo='https://github.com/'+repo
        sha=meta.get('gitHead')
        if not sha:raise ValueError('no gitHead '+name)
        if name in ('is-reference','locate-character'):
            docs.append(doc('Published license declaration', 'License: '+p['license']+'\nAuthors (package metadata): '+str(package.get('author',''))+'\nFull standard terms are included separately.\n',p['resolved']+' :: package.json'))
        elif 'github.com/' in repo:
            docs+=upstream_docs(repo,sha,'packages/'+name.split('/')[-1])
        else:
            url=repo+'/-/raw/'+sha+'/LICENSE'
            docs.append(doc('LICENSE',get(url),url))
    if not has_terms(docs) and name not in ('is-reference','locate-character'):raise ValueError('missing full license '+name)
    return {'ecosystem':'npm','name':name,'version':p['version'],'license':p['license'],'source':p['resolved'],'integrity':p['integrity'],'documents':docs}

def collect_native():
    p=PACKAGES['webview2-com-sys']
    d=pathlib.Path(p['manifest_path']).parent
    sdk=re.search(r'update WebView2 SDK to ([0-9.]+)',(d/'CHANGELOG.md').read_text()).group(1)
    url=f'https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/{sdk}/microsoft.web.webview2.{sdk}.nupkg'
    data=get(url)
    archive=zipfile.ZipFile(io.BytesIO(data))
    hashes={}
    for arch in ['x64','x86','arm64']:
        original=archive.read(f'build/native/{arch}/WebView2LoaderStatic.lib')
        if original != (d/arch/'WebView2LoaderStatic.lib').read_bytes():
            raise ValueError('WebView2 loader differs from Microsoft SDK: '+arch)
        hashes[arch]=hashlib.sha256(original).hexdigest()
    return [{'name':'Microsoft.Web.WebView2 SDK / WebView2Loader','version':sdk,'license':'BSD-3-Clause',
        'source':url,'archive_sha256':hashlib.sha256(data).hexdigest(),
        'via':{'name':p['name'],'version':p['version']},'binary_sha256':hashes,
        'documents':[doc(name,archive.read(name),url+' :: '+name) for name in ['LICENSE.txt','NOTICE.txt']]}]

LOCK=json.load(open(ROOT/'package-lock.json'))
items=[]; errors=[]
jobs=[('cargo '+p['name'],lambda p=p:collect_rust(p)) for p in META['packages'] if p['name']!='netmon']
jobs += [('npm '+k,lambda k=k,p=p:collect_npm(k,p)) for k,p in LOCK['packages'].items() if k]
def run(job):
    name,fn=job
    try:return fn()
    except Exception as e:return {'error':name+': '+str(e)}
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
    for result in pool.map(run,jobs):
        if 'error' in result:errors.append(result['error']);print(result['error'],flush=True)
        else:items.append(result)
if errors:
    raise SystemExit(f'Collection incomplete ({len(errors)} errors); existing inventory preserved')
items.sort(key=lambda p:(p['ecosystem'],p['name'],p['version']))
standard={}
ids=set()
for item in items:
    ids.update(re.findall(r'[A-Za-z0-9][A-Za-z0-9.-]*',item['license']))
for id in sorted(ids-{'AND','OR','WITH'}):
    folder='text'
    url=f'https://raw.githubusercontent.com/spdx/license-list-data/v3.27.0/{folder}/{id}.txt'
    standard[id]=doc(id,get(url),url)
inventory={'standard_terms':standard,'lockfiles':{path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in ('package-lock.json','src-tauri/Cargo.lock')},'schema':1,'scope':'All locked Cargo and npm dependencies, including build tools and optional platforms. Not all are linked or shipped on each platform.','packages':items}
native=collect_native()
atomic_write(OUT/'native.json', (json.dumps(native,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
atomic_write(OUT/'inventory.json', (json.dumps(inventory,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
print(f'Collected {len(items)} packages; {len(errors)} errors',flush=True)
if errors:raise SystemExit(1)
