"""Fetch the small public quality corpus, or verify its existing cache with --reuse.
Requires Python3 and an existing ffmpeg; installs nothing. No accounts/tokens.
Usage: python scripts/fetch-quality-corpus.py [--reuse]
Outputs repo/.context/quality-corpus; audio transfer is capped at10,000,000bytes.
Attribution/results: docs/benchmarks/2026-10-21-human-corpus.md.
"""
import array
import hashlib
import json
import math
import pathlib
import shutil
import subprocess
import sys
import urllib.parse
import urllib.request
import wave

ROOT = pathlib.Path(__file__).resolve().parent.parent / '.context' / 'quality-corpus'
BUDGET = 10_000_000

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def write_json(name, value):
    (ROOT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')

def read_audio(entry):
    with wave.open(str(ROOT / entry['wav']), 'rb') as audio:
        assert audio.getnchannels() == 1 and audio.getframerate() == 16000 and audio.getsampwidth() == 2
        values = array.array('h', audio.readframes(audio.getnframes()))
        if sys.byteorder != 'little': values.byteswap()
        return [v / 32768.0 for v in values]

def save_audio(name, samples):
    values = array.array('h', [int(max(-1, min(1, v)) * 32767) for v in samples])
    if sys.byteorder != 'little': values.byteswap()
    with wave.open(str(ROOT / name), 'wb') as audio:
        audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(16000)
        audio.writeframes(values.tobytes())
    return digest(ROOT / name)

def derive(entries):
    results = list(entries)
    for entry in entries:
        clean = read_audio(entry); seed = 0x4c4543545553; noise = []
        for _ in clean:
            seed = (seed * 6364136223846793005 + 1) & 0xffffffffffffffff
            noise.append(((seed >> 32) / 0xffffffff) * 2 - 1)
        scale = math.sqrt(sum(v*v for v in clean) / (sum(v*v for v in noise) * 10))
        mixed = [s+n*scale for s,n in zip(clean, noise)]
        peak = max(1, max(abs(v) for v in mixed))
        item = dict(entry); item['id'] += '-synthetic-noise10db'; item['wav'] = item['id']+'.wav'
        item['wav_sha256'] = save_audio(item['wav'], [v/peak for v in mixed])
        item['provenance'] += '; DERIVATION deterministic uniform noise10dB,LCG seed4c4543545553; not natural environmental noise'
        results.append(item)
    for language in ['zh', 'en', 'pt']:
        group = [e for e in entries if e['language'] == language]
        clips = [read_audio(e) for e in group]; samples = []; references = []; order = []; index = 0
        while len(samples)+len(clips[index%2]) <= 16000*60:
            samples += clips[index%2]; references.append(group[index%2]['reference'])
            order.append(group[index%2]['id']); index += 1
        shorter = min(range(2), key=lambda k: len(clips[k]))
        while len(samples)+len(clips[shorter]) <= 16000*60:
            samples += clips[shorter]; references.append(group[shorter]['reference']); order.append(group[shorter]['id'])
        item = dict(group[0]); item['id'] = language+'-synthetic-long-public'; item['wav'] = item['id']+'.wav'
        item['reference'] = ' '.join(references); item['wav_sha256'] = save_audio(item['wav'], samples)
        item['provenance'] = 'Google FLEURS CC-BY4.0; synthetic concatenation of complete untrimmed human-read clips; order='+','.join(order)+'; original references concatenated; no new speakers'
        results.append(item)
    return results

def fetch():
    if not shutil.which('ffmpeg'): raise RuntimeError('ffmpeg is required; this script will not install it')
    ROOT.mkdir(parents=True, exist_ok=True)
    if any((ROOT/name).exists() for name in ['manifest-clean.json','manifest-ptpt.json','manifest-benchmark.json']):
        raise RuntimeError('Existing corpus manifests found. Use --reuse to verify instead of overwriting them.')
    transferred = 0; fleurs = []; minds = []
    specs = [('google/fleurs','cmn_hans_cn','validation','zh','cmn-Hans-CN'),
             ('google/fleurs','en_us','validation','en','en-US'),
             ('google/fleurs','pt_br','validation','pt','pt-BR'),
             ('PolyAI/minds14','pt-PT','train','pt','pt-PT')]
    for dataset,config,split,language,locale in specs:
        query = urllib.parse.urlencode(dict(dataset=dataset,config=config,split=split,offset=0,length=2))
        canonical = 'https://datasets-server.huggingface.co/rows?'+query
        with urllib.request.urlopen(canonical, timeout=25) as response:
            metadata = json.loads(response.read(2_000_001))
        if len(metadata['rows']) != 2: raise RuntimeError('Expected two public rows')
        for record in metadata['rows']:
            row = record['row']; index = record['row_idx']
            if dataset == 'PolyAI/minds14':
                languages = next(f['type']['names'] for f in metadata['features'] if f['name']=='lang_id')
                assert languages[row['lang_id']] == 'pt-PT' and row['path'].startswith('pt-PT~')
                stem = 'minds-pt-PT-train-'+str(index)
            else: stem = config+'-validation-'+str(index)
            original = ROOT/(stem+'.wav'); converted = ROOT/(stem+'-pcm16.wav')
            if original.exists() or converted.exists(): raise RuntimeError('Existing audio found; inspect cache or use --reuse')
            # The ephemeral signed public asset URL exists only in memory. It
            # is neither logged nor written to portable metadata.
            with urllib.request.urlopen(row['audio'][0]['src'], timeout=30) as response:
                length = int(response.headers.get('Content-Length','0'))
                if length <= 0 or length > BUDGET-transferred: raise RuntimeError('Audio exceeds remaining10MB budget or has unknown length')
                with original.open('wb') as output:
                    count = 0
                    while count < length:
                        block = response.read(min(65536,length-count))
                        if not block: break
                        output.write(block); count += len(block)
                transferred += count
                if count != length: raise RuntimeError('Public audio transfer was incomplete; no manifest published')
            subprocess.run(['ffmpeg','-v','error','-nostdin','-i',str(original),'-ar','16000','-ac','1','-c:a','pcm_s16le',str(converted)], check=True)
            source = ('Google FLEURS human read speech; official written reading reference' if dataset=='google/fleurs' else
                      'PolyAI MINDS14 human pt-PT speech; reference=published Google ASR hypothesis, NOT human gold; English text=machine translation')
            item = dict(id=stem+('-id'+str(row['id']) if 'id' in row else ''),wav=converted.name,language=language,
                        locale=locale,reference=row['transcription'],provenance=source+'; CC-BY4.0; '+canonical+'; row='+str(index)+'; converted toPCM16mono16k; individual accent not independently assessed',
                        source_url=canonical,license='CC-BY-4.0',source_wav_sha256=digest(original),wav_sha256=digest(converted),downloaded_bytes=count)
            if 'english_transcription' in row: item['english_transcription']=row['english_transcription']
            (fleurs if dataset=='google/fleurs' else minds).append(item)
            print(stem+': '+str(count)+' public audio bytes')
    write_json('manifest-clean.json',fleurs); write_json('manifest-ptpt.json',minds)
    write_json('manifest-benchmark.json',derive(fleurs))
    print('Public audio transferred: '+str(transferred)+' bytes; no signed asset URLs saved')

def reuse():
    checked = set()
    for name in ['manifest-clean.json','manifest-ptpt.json','manifest-benchmark.json']:
        for entry in json.loads((ROOT/name).read_text(encoding='utf-8')):
            path = ROOT/entry['wav']
            if path not in checked:
                assert digest(path)==entry['wav_sha256'], 'Cached corpus checksum mismatch'
                read_audio(entry); checked.add(path)
    print('Verified '+str(len(checked))+' PCM16mono16k cached WAVs; no network requests or inference')

if __name__ == '__main__':
    if sys.argv[1:] == ['--reuse']: reuse()
    elif not sys.argv[1:]: fetch()
    else: raise SystemExit(__doc__)
