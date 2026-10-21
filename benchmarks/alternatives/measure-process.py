#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys, time
root=pathlib.Path(__file__).resolve().parent
name=sys.argv[1]
command=sys.argv[2:]
env=os.environ.copy()
env['LD_LIBRARY_PATH']=str(root/'sysroot/usr/lib/x86_64-linux-gnu')
env['ALSA_CONFIG_PATH']=str(root/'alsa-null.conf')
log_path=root/(name+'-runtime.log')
with log_path.open('w') as log:
  proc=subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
  samples=[]
  try:
    for sec in range(1,7):
      time.sleep(1)
      if proc.poll() is not None: break
      stats={}
      for line in pathlib.Path(f'/proc/{proc.pid}/smaps_rollup').read_text().splitlines()[1:]:
        fields=line.split()
        if fields and fields[0] in ['Rss:','Pss:','Private_Clean:','Private_Dirty:']:
          stats[fields[0][:-1]+'_KiB']=int(fields[1])
      status=pathlib.Path(f'/proc/{proc.pid}/status').read_text()
      stats['threads']=int(next(x.split()[1] for x in status.splitlines() if x.startswith('Threads:')))
      stat=pathlib.Path(f'/proc/{proc.pid}/stat').read_text().split()
      stats['cpu_seconds']=(int(stat[13])+int(stat[14]))/os.sysconf('SC_CLK_TCK')
      stats['elapsed_seconds']=sec
      samples.append(stats)
    result={'engine':name,'platform':'Linux x86_64','state':'no authenticated session, no Spotify audio','command':command,'samples':samples,'exited_before_stop':proc.poll()}
  finally:
    if proc.poll() is None:
      proc.terminate()
      try: proc.wait(timeout=2)
      except subprocess.TimeoutExpired:
        proc.kill(); proc.wait(timeout=2)
result_path=root/(name+'-measurement.json')
result_path.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
