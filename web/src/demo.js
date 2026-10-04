(function(){
'use strict';
/* Demo control panel. Plays the step scripts the build baked; decides nothing itself. */
const S=JSON.parse(document.getElementById('ess-sim').textContent);
const root=document.getElementById('demo');if(!root)return;
const reduce=window.matchMedia('(prefers-reduced-motion: reduce)');
const $=(s,r)=>(r||document).querySelector(s),$$=(s,r)=>Array.from((r||document).querySelectorAll(s));
const short=q=>String(q==null?'':q).split('.').pop();
const slug=s=>String(s).replace(/[^A-Za-z0-9_-]/g,'_');
const escH=s=>String(s==null?'':s).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
const NS='http://www.w3.org/2000/svg';
const svg=$('#dm-canvas'),tokL=$('#dm-tokens'),ov=$('#dm-overlay');
const byId=Object.fromEntries(S.traces.map(t=>[t.id,t]));
const PAL=['#1f8fb3','#c27a1a','#7d63d9','#23926a','#c94a6d','#3f7acc','#a8890f','#b8558a','#4d8f2c','#6574c9','#b85a2f','#2c8a85'];
function hue(s){let h=0;for(const c of String(s))h=(h*31+c.charCodeAt(0))>>>0;return PAL[h%PAL.length];}
const st={trace:null,k:0,playing:false,speed:1,timer:null,gen:0,follow:true,sel:null,dock:'views'};

/* ---------- state at step k: the fold of the baked deltas ---------- */
function stateAt(tr,k){const inst=new Map(),views={},events=[],hist=new Map();let at=null;
 for(let i=0;i<k&&i<tr.steps.length;i++){const s=tr.steps[i];
  (s.d||[]).forEach(([e,id,b,a,t,f])=>{if(a==null){inst.delete(id);return;}
   const cur=inst.get(id)||{entity:e,born:i,label:(tr.labels||{})[id]||short(e)+' '+String(id).slice(-4)};
   cur.state=a;cur.fields=f||{};inst.set(id,cur);if(!hist.has(id))hist.set(id,[]);if(b!==a)hist.get(id).push([i,a,t]);});
  if(s.cap){const c=inst.get(s.cap[0]);if(c)c.label=s.cap[1];}
  Object.entries(s.vs||{}).forEach(([v,rows])=>views[v]={rows,step:i});
  (s.ev||[]).forEach(([e,p])=>events.push({i,e,p}));
  if(s.at)at=s.at;}
 return {inst,views,events,hist,at};}

/* ---------- camera ---------- */
const full=svg.dataset.full.split(' ').map(Number);let vb=full.slice(),camAnim=0;
function setVB(v){vb=v;svg.setAttribute('viewBox',v.map(x=>x.toFixed(1)).join(' '));}
function fitVB(){const r=svg.getBoundingClientRect(),ar=(r.width||1)/(r.height||1);let x,y,w,h;try{const b=svg.querySelector('.dm-body').getBBox();x=b.x-16;y=b.y-12;w=b.width+32;h=b.height+24;}catch(e){[x,y,w,h]=full;}if(!(w>0&&h>0))[x,y,w,h]=full;
 if(w/h>ar){const nh=w/ar;y-=(nh-h)/2;h=nh;}else{const nw=h*ar;x-=(nw-w)/2;w=nw;}return[x,y,w,h];}
function ease(to,ms){cancelAnimationFrame(camAnim);if(reduce.matches||!ms){setVB(to);return;}const from=vb.slice(),t0=performance.now();
 const tick=now=>{const k=Math.min(1,(now-t0)/ms),e=k<.5?2*k*k:1-Math.pow(-2*k+2,2)/2;setVB(from.map((f,i)=>f+(to[i]-f)*e));if(k<1)camAnim=requestAnimationFrame(tick);};camAnim=requestAnimationFrame(tick);}
function frameOf(e){return svg.querySelector('.dm-frame[data-me="'+CSS.escape(e)+'"]');}
/* follow: the whole component column that holds the entity, padded, is inside the viewport; the
   view only ever grows from that box to match the canvas's aspect ratio, never shrinks below it */
function focusBox(f){let x=+f.dataset.x,y=+f.dataset.y,w=+f.dataset.w,h=+f.dataset.h;
 const col=[...svg.querySelectorAll('.dm-colbox')].find(b=>{const bx=+b.getAttribute('x'),bw=+b.getAttribute('width');return bx<=x+.5&&x+w<=bx+bw+.5;});
 if(col){const bx=+col.getAttribute('x'),by=+col.getAttribute('y'),bw=+col.getAttribute('width'),bh=+col.getAttribute('height');
  const x2=Math.max(x+w,bx+bw),y2=Math.max(y+h,by+bh);x=Math.min(x,bx);y=Math.min(y,by);w=x2-x;h=y2-y;}
 return[x,y,w,h];}
function focusFrame(e,ms){const f=frameOf(e);if(!f)return false;const[x,y,w,h]=focusBox(f),P=24;
 const r=svg.getBoundingClientRect(),ar=(r.width||1)/(r.height||1);let W=Math.max(w+2*P,360),H=Math.max(h+2*P,240);
 if(W/H>ar)H=W/ar;else W=H*ar;
 /* when the whole canvas fits at nearly the same scale, show all of it rather than half a neighbour */
 const fit=fitVB(),to=fit[2]<=W/0.85?fit:[x+w/2-W/2,y+h/2-H/2,W,H];
 ease(to,ms==null?700/Math.max(1,st.speed/2):ms);return true;}
/* the entity the step at k acted on: the last command at or before it that changed or refused one */
function focusAt(k){const tr=st.trace;if(!tr)return null;for(let i=Math.min(k,tr.steps.length)-1;i>=0;i--){const s=tr.steps[i];if(s.k!=='execute_command')continue;
 const e=((s.d||[])[0]||[])[0]||s.se;if(e&&frameOf(e))return e;}return null;}
function reframe(ms){const e=st.follow&&focusAt(st.k);if(!(e&&focusFrame(e,ms)))ease(fitVB(),ms||0);}
function toSvg(evt){const pt=svg.createSVGPoint();pt.x=evt.clientX;pt.y=evt.clientY;return pt.matrixTransform(svg.getScreenCTM().inverse());}
svg.addEventListener('wheel',e=>{e.preventDefault();const p=toSvg(e),f=Math.exp(e.deltaY*0.0015);const[x,y,w,h]=vb;
 setVB([p.x-(p.x-x)*f,p.y-(p.y-y)*f,w*f,h*f]);},{passive:false});
const ptrs=new Map();let drag=null;
svg.addEventListener('pointerdown',e=>{if(e.target.closest('.tok,a'))return;svg.setPointerCapture(e.pointerId);ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});
 drag={vb:vb.slice(),x:e.clientX,y:e.clientY,d:null};svg.classList.add('dragging');});
svg.addEventListener('pointermove',e=>{if(!ptrs.has(e.pointerId))return;ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});
 const r=svg.getBoundingClientRect(),sc=vb[2]/(r.width||1);
 if(ptrs.size===2){const[a,b]=[...ptrs.values()];const d=Math.hypot(a.x-b.x,a.y-b.y);if(!drag.d){drag.d=d;drag.vb=vb.slice();return;}
  const f=drag.d/d,[x,y,w,h]=drag.vb;setVB([x+w*(1-f)/2,y+h*(1-f)/2,w*f,h*f]);return;}
 if(drag&&ptrs.size===1){const[x,y,w,h]=drag.vb;setVB([x-(e.clientX-drag.x)*sc,y-(e.clientY-drag.y)*sc,w,h]);}});
['pointerup','pointercancel'].forEach(t=>svg.addEventListener(t,e=>{ptrs.delete(e.pointerId);if(!ptrs.size){drag=null;svg.classList.remove('dragging');}}));

/* ---------- tokens ---------- */
function nodeOf(e,s){return svg.querySelector('[data-ms="'+CSS.escape(e+'#'+s)+'"]');}
/* tokens sit in a row to the right of their state pill; one label per state lists the instances */
function slots(state){const groups=new Map();[...state.inst.entries()].sort((a,b)=>a[1].born-b[1].born).forEach(([id,x])=>{
 const key=x.entity+'#'+x.state;if(!groups.has(key))groups.set(key,[]);groups.get(key).push(id);});
 const pos=new Map(),labels=[];groups.forEach((ids,key)=>{const i0=key.lastIndexOf('#'),n=nodeOf(key.slice(0,i0),key.slice(i0+1));if(!n)return;
  const cx=+n.dataset.cx,cy=+n.dataset.cy,body=n.querySelector('.body'),w=body?+body.getAttribute('width'):90,x0=cx+w/2+(n.classList.contains('terminal')?14:10);
  ids.forEach((id,i)=>pos.set(id,[x0+i*13,cy,i]));
  const names=ids.map(id=>state.inst.get(id).label),text=names.length>3?names.slice(0,3).join(', ')+' +'+(names.length-3):names.join(', ');
  let lx=x0+(ids.length-1)*13+9,ly=cy+3.5;const lw=text.length*6.2,frame=n.closest('.dm-frame');
  /* a label that would run into a neighbouring state pill goes under the row of dots instead */
  const hit=frame&&[...frame.querySelectorAll('.state')].some(o=>{if(o===n)return false;const b=o.querySelector('.body');if(!b)return false;
   const ox=+b.getAttribute('x'),ow=+b.getAttribute('width'),oy=+b.getAttribute('y'),oh=+b.getAttribute('height');
   return ox<lx+lw&&ox+ow>lx&&oy<ly+2&&oy+oh>ly-9;});
  if(hit){lx=x0-5;ly=cy+19;}
  labels.push([lx,ly,text,ids]);});
 return {pos,labels};}
function tokenEl(id,x){let g=tokL.querySelector('[data-tok="'+CSS.escape(id)+'"]');if(g)return g;
 g=document.createElementNS(NS,'g');g.setAttribute('class','tok');g.dataset.tok=id;const dc=(S.domains[(S.entity[x.entity]||{}).domain]||{}).colour||'#1f8fb3';
 g.style.setProperty('--dc',dc);g.innerHTML='<circle class="halo" r="9"/><circle class="core" r="5.5"/><title></title><text class="err" x="-4" y="-11"></text>';
 g.addEventListener('click',ev=>{ev.stopPropagation();st.sel=id;renderDock();$$('.tok.sel').forEach(t=>t.classList.remove('sel'));g.classList.add('sel');showDock('inst');});
 tokL.appendChild(g);return g;}
function place(g,x,y){g.setAttribute('transform','translate('+x.toFixed(1)+','+y.toFixed(1)+')');}
function renderTokens(state){const{pos,labels}=slots(state);const alive=new Set(state.inst.keys());
 $$('.tok',tokL).forEach(g=>{if(!alive.has(g.dataset.tok))g.remove();});$$('.toklab',tokL).forEach(t=>t.remove());
 state.inst.forEach((x,id)=>{const g=tokenEl(id,x),p=pos.get(id);if(!p)return;place(g,p[0],p[1]);
  g.querySelector('title').textContent=x.label+' · '+x.entity+' · '+x.state+' · '+id;g.classList.toggle('sel',st.sel===id);});
 labels.forEach(([x,y,text])=>{const t=document.createElementNS(NS,'text');t.setAttribute('class','toklab');t.setAttribute('x',x.toFixed(1));t.setAttribute('y',y.toFixed(1));
  t.textContent=text;tokL.insertBefore(t,tokL.firstChild);});}

/* ---------- cause and effect, particles ---------- */
function scr(el){const r=el.getBoundingClientRect();return[r.left+r.width/2,r.top+r.height/2];}
function svgScr(x,y){const m=svg.getScreenCTM();return[m.a*x+m.c*y+m.e,m.b*x+m.d*y+m.f];}
function cause(s,ents){if(reduce.matches)return;ov.innerHTML='';const pts=[];
 const a=s.a&&$('.dm-actor[data-actor="'+CSS.escape(s.a)+'"]');if(a)pts.push(scr(a));
 (s.comp||[]).slice(0,1).forEach(c=>{const h=svg.querySelector('[data-comp-head="'+CSS.escape(c)+'"]');if(h)pts.push(scr(h));});
 const f=ents.map(frameOf).find(Boolean);if(f)pts.push(svgScr(+f.dataset.x+(+f.dataset.w)/2,+f.dataset.y+12));
 if(pts.length<2)return;const p=document.createElementNS(NS,'polyline');p.setAttribute('class','cause');p.setAttribute('points',pts.map(q=>q.join(',')).join(' '));
 ov.appendChild(p);p.animate([{opacity:0},{opacity:.95},{opacity:.95},{opacity:0}],{duration:1500/st.speed,fill:'forwards'});}
function sparks(s,from){if(reduce.matches||!(s.ev||[]).length||!from)return;const tab=$('.dm-tabs [data-dock="events"]');if(!tab)return;const to=scr(tab);
 s.ev.forEach((e,i)=>{const c=document.createElementNS(NS,'circle');c.setAttribute('class','spark');c.setAttribute('r','4.5');ov.appendChild(c);
  const dur=900/st.speed;c.animate([{transform:'translate('+from[0]+'px,'+from[1]+'px)',opacity:1},{transform:'translate('+to[0]+'px,'+to[1]+'px)',opacity:.2}],
   {duration:dur,delay:i*120/st.speed,easing:'ease-in',fill:'forwards'}).onfinish=()=>c.remove();});}

/* ---------- dock, lanes, pass panel ---------- */
function showDock(name){st.dock=name;$$('.dm-tabs [data-dock]').forEach(b=>b.classList.toggle('on',b.dataset.dock===name));
 $$('.dm-panel').forEach(p=>p.hidden=p.dataset.panel!==name);renderDock();}
$$('.dm-tabs [data-dock]').forEach(b=>b.addEventListener('click',()=>showDock(b.dataset.dock)));
function val(v){return v==null?'':(typeof v==='object'?JSON.stringify(v):String(v));}
function renderDock(){const tr=st.trace,state=stateAt(tr,st.k),cur=tr.steps[st.k-1];
 const changed=new Set(((cur||{}).d||[]).map(d=>d[1]));
 if(st.dock==='views'){$('#dm-views').innerHTML=Object.keys(S.views).sort().map(v=>{const vi=S.views[v],x=state.views[v],rows=x?x.rows:[];
  const head='<tr>'+vi.fields.map(f=>'<th>'+escH(f)+'</th>').join('')+'</tr>';
  const body=rows.map(([key,row])=>'<tr class="'+(changed.has(key)&&x.step===st.k-1?'chg':'')+'">'+vi.fields.map(f=>'<td title="'+escH(val(row[f]))+'">'+escH(val(row[f]))+'</td>').join('')+'</tr>').join('');
  return '<div class="dm-view" data-dom="'+escH(vi.domain)+'"><h5><a href="#'+escH(vi.a)+'" data-model-ref="'+escH(vi.a)+'">'+escH(short(v))+'</a> <span class="dim">'+rows.length+' row(s)</span></h5>'+
   (rows.length?'<div class="tscroll"><table>'+head+body+'</table></div>':'<div class="none">no rows</div>')+'</div>';}).join('');}
 if(st.dock==='events'){const dom=$('#dm-evdom').value;$('#dm-events').innerHTML=state.events.slice().reverse().filter(x=>!dom||S.events[x.e]===dom).slice(0,300)
  .map(x=>'<li class="'+(x.i===st.k-1?'cur':'')+'" data-dom="'+escH(S.events[x.e]||'')+'"><span class="n">#'+(x.i+1)+'</span> <b>'+escH(short(x.e))+'</b> <code>'+escH(JSON.stringify(x.p))+'</code></li>').join('')||'<li class="na">no events yet</li>';}
 if(st.dock==='checks'){$('#dm-checks').innerHTML=tr.steps.map((s,i)=>s.k==='execute_command'?'':'<li data-seek="'+(i+1)+'" class="'+(s.u?'na':s.ok?'ok':s.ok===false?'bad':'na')+(i===st.k-1?' cur':'')+'">'+
  '<span class="n">#'+(i+1)+' '+escH(s.k)+'</span> '+(s.u?'not evaluable':s.ok?'met':s.ok===false?'<b>unmet</b>':'')+'<br>'+escH(s.t||'')+'</li>').join('');}
 if(st.dock==='seq')renderSeq(tr);
 if(st.dock==='inst'){const x=st.sel&&state.inst.get(st.sel);if(!x){$('#dm-inst').innerHTML='<span class="dim">Click a token on the canvas.</span>';}
  else{const h=(state.hist.get(st.sel)||[]).map(([i,s,t])=>'<li><span class="n">#'+(i+1)+'</span> '+escH(t||'created')+' → <b>'+escH(s)+'</b></li>').join('');
   $('#dm-inst').innerHTML='<div class="ik">'+escH(short(x.entity))+'</div><h3>'+escH(x.label)+'</h3><div class="iid">'+escH(st.sel)+'</div><dl><dt>entity</dt><dd><a href="#'+escH((S.entity[x.entity]||{}).a)+'" data-model-ref="'+escH((S.entity[x.entity]||{}).a)+'">'+escH(x.entity)+'</a></dd><dt>state</dt><dd>'+escH(x.state)+'</dd>'+
    Object.entries(x.fields||{}).map(([k,v])=>'<dt>'+escH(k)+'</dt><dd>'+escH(val(v))+'</dd>').join('')+'</dl><ol class="dm-feed">'+h+'</ol>';}}}
$('#dm-evdom').addEventListener('change',renderDock);
$('#dm-checks').addEventListener('click',e=>{const li=e.target.closest('[data-seek]');if(li){pause();seek(+li.dataset.seek);}});
function renderSeq(tr){const g=$('#dm-seq');const upto=tr.steps.slice(0,st.k).map((s,i)=>[s,i]).filter(([s])=>s.k==='execute_command').slice(-12);
 const parts=[];upto.forEach(([s])=>{const a=s.a?short(s.a):'(no actor)';if(!parts.includes(a))parts.push(a);(s.comp||[]).forEach(c=>{if(!parts.includes(c))parts.push(c);});});
 if(!upto.length){g.innerHTML='<text x="10" y="20">no commands yet</text>';g.setAttribute('viewBox','0 0 340 40');return;}
 const W=Math.max(340,parts.length*120),X=p=>60+parts.indexOf(p)*((W-120)/Math.max(1,parts.length-1)),H=40+upto.length*38+10;
 let h=parts.map(p=>'<text class="hd" x="'+X(p)+'" y="16" text-anchor="middle">'+escH(p)+'</text><line class="ll" x1="'+X(p)+'" y1="24" x2="'+X(p)+'" y2="'+H+'"/>').join('');
 upto.forEach(([s,i],r)=>{const y=40+r*38,a=s.a?short(s.a):'(no actor)',c=(s.comp||[])[0]||a,x1=X(a),x2=X(c),cur=i===st.k-1?' cur':'';
  h+='<g class="'+cur+'"><line class="msg" x1="'+x1+'" y1="'+y+'" x2="'+x2+'" y2="'+y+'" marker-end="url(#arrow-unconditional)"/><text x="'+((x1+x2)/2)+'" y="'+(y-4)+'" text-anchor="middle">'+escH(short(s.c))+'</text>'+
   '<line class="ret'+(s.e||!s.o?' err':'')+'" x1="'+x2+'" y1="'+(y+14)+'" x2="'+x1+'" y2="'+(y+14)+'" marker-end="url(#arrow-link)"/><text x="'+((x1+x2)/2)+'" y="'+(y+26)+'" text-anchor="middle">'+escH(s.e?short(s.e):(s.o||'no declared outcome'))+'</text></g>';});
 g.setAttribute('viewBox','0 0 '+W+' '+H);g.innerHTML=h;}
function renderLanes(tr){const g=$('#dm-lanes'),N=tr.steps.length,stepW=Math.max(8,Math.min(22,1400/Math.max(1,N))),L=140;
 const final=stateAt(tr,N);const ids=[];const seen=new Set();tr.steps.forEach(s=>(s.d||[]).forEach(d=>{if(!seen.has(d[1])){seen.add(d[1]);ids.push([d[1],d[0]]);}}));
 const lab={};const full=stateAt(tr,N);ids.forEach(([id,e])=>{lab[id]=((full.inst.get(id)||{}).label)||(tr.labels||{})[id]||short(e);});
 const W=L+N*stepW+20,H=16+ids.length*14;let h='';
 ids.forEach(([id,e],r)=>{const y=10+r*14;h+='<text x="4" y="'+(y+10)+'">'+escH(lab[id])+'</text>';let cur=null,from=0;const segs=[];
  for(let i=0;i<=N;i++){let s=null;if(i<N){for(const d of tr.steps[i].d||[])if(d[1]===id)cur=d[3];s=cur;}
   if(i===N||s!==segs.st){if(segs.st!=null)h+='<rect class="seg" x="'+(L+from*stepW)+'" y="'+y+'" width="'+((i-from)*stepW)+'" height="12" fill="'+hue(segs.st)+'"><title>'+escH(lab[id]+': '+segs.st)+'</title></rect>'+
     ((i-from)*stepW>60?'<text x="'+(L+from*stepW+3)+'" y="'+(y+10)+'" style="fill:#fff">'+escH(segs.st)+'</text>':'');segs.st=s;from=i;}}
  tr.steps.forEach((s,i)=>{if(s.si===id&&(s.ref&&(s.e||!s.o)))h+='<circle class="refm" cx="'+(L+i*stepW+stepW/2)+'" cy="'+(y+6)+'" r="3.5"><title>'+escH('#'+(i+1)+' '+(s.e||'refused'))+'</title></circle>';});});
 h+='<line class="cursor" id="dm-cursor" x1="0" y1="0" x2="0" y2="'+H+'"/>';g.setAttribute('viewBox','0 0 '+W+' '+H);g.setAttribute('width',W);g.setAttribute('height',H);g.innerHTML=h;g.dataset.l=L;g.dataset.sw=stepW;
 void final;}
function moveCursor(){const g=$('#dm-lanes'),c=$('#dm-cursor');if(!c)return;const x=+g.dataset.l+st.k*(+g.dataset.sw);c.setAttribute('x1',x);c.setAttribute('x2',x);}
$('#dm-lanes').addEventListener('click',e=>{const g=$('#dm-lanes'),r=g.getBoundingClientRect(),vbw=+g.getAttribute('width');const x=(e.clientX-r.left)*vbw/r.width;
 const k=Math.round((x-(+g.dataset.l))/(+g.dataset.sw));pause();seek(Math.max(0,Math.min(st.trace.steps.length,k)));});
function renderPass(){const tr=st.trace,box=$('#dm-pass');if(st.k<tr.steps.length){box.hidden=true;return;}
 const vx=tr.steps.filter(s=>s.k==='expect_view'||s.k==='eventually_view'),all=tr.steps.filter(s=>s.k!=='execute_command'&&!s.u&&s.ok!=null&&/^(expect|eventually)/.test(s.k));
 const met=all.filter(s=>s.ok).length;box.hidden=false;
 box.innerHTML='<h4>'+escH(tr.title)+' · end</h4><div class="summ">'+met+' of '+all.length+' expectations met by the interpreter</div>'+
  (vx.length?'<ol>'+vx.map(s=>'<li class="'+(s.ok?'good':'bad')+'">'+(s.ok?'✓ met':'✗ unmet')+' — '+escH(s.t)+'</li>').join('')+'</ol>':'');}

/* ---------- run picker: a combobox over every baked scenario and run ----------
 The button names the current run, its kind and its length; the popover lists every run grouped by kind, filtered by
 the search field, each with its step count, its verdict (icon and word) and its one-line purpose. Keyboard: ↑/↓, Home/End,
 Enter, Esc, Tab; typing on the button opens the list with that text. Below 800px wide it is a bottom sheet. */
const picker=(function(){
 const wrap=$('#dm-run');if(!wrap)return{set(){},show(){},hide(){},isOpen:()=>false};
 const btn=$('#dm-trace'),pop=$('#dm-run-pop'),scrim=$('.rp-scrim',wrap),q=$('#dm-run-q'),list=$('#dm-run-list'),count=$('.rp-count',wrap);
 const KIND={authored:'authored',synthesized:'synthesized',seeded:'seeded run'};
 const GROUPS=[['authored','Authored scenarios'],['synthesized','Synthesized scenarios'],['seeded','Seeded runs']];
 const VERD={met:['✓','met'],failed:['✕','failed'],undetermined:['?','undetermined']};
 const origin=t=>t.origin||(t.kind==='run'?'seeded':'synthesized');
 const label=t=>{if(origin(t)!=='synthesized')return t.title;const p=t.title.split('/');return short(p[0])+' › '+p.slice(1).join(' / ');};
 const desc=t=>(t.summary||'').replace(/`/g,'');
 let shown=[],active=-1,open=false,cur=null;
 function set(tr){cur=tr;$('.rp-name',btn).textContent=label(tr);const k=$('.rp-kind',btn);k.textContent=KIND[origin(tr)];k.className='rp-kind k-'+origin(tr);
  $('.rp-steps',btn).textContent=tr.steps.length+' steps';btn.title=tr.title+(tr.summary?' — '+desc(tr):'')+' (r)';}
 function row(t,i){const v=t.verdict&&VERD[t.verdict],on=!!(cur&&t.id===cur.id);
  return '<div class="rp-opt'+(on?' cur':'')+'" role="option" id="rp-o-'+i+'" data-i="'+i+'" aria-selected="'+on+'">'+
   '<span class="rp-o-main" title="'+escH(t.title)+'"><span class="rp-o-name">'+escH(label(t))+'</span>'+(t.summary?'<span class="rp-o-desc">'+escH(desc(t))+'</span>':'')+'</span>'+
   '<span class="rp-o-meta">'+(v?'<span class="rp-v v-'+t.verdict+'"><i aria-hidden="true">'+v[0]+'</i>'+v[1]+'</span>':'')+
   '<span class="rp-o-steps">'+t.steps.length+' steps</span></span></div>';}
 function draw(){const term=q.value.trim().toLowerCase();shown=[];let h='';
  GROUPS.forEach(([g,label])=>{const ts=S.traces.filter(t=>origin(t)===g&&(!term||(t.title+' '+(t.summary||'')+' '+(t.group||'')).toLowerCase().includes(term)));
   if(!ts.length)return;h+='<div class="rp-group" role="group" aria-labelledby="rp-g-'+g+'"><div class="rp-g" id="rp-g-'+g+'">'+label+'<b>'+ts.length+'</b></div>';
   ts.forEach(t=>{h+=row(t,shown.length);shown.push(t);});h+='</div>';});
  list.innerHTML=h||'<div class="rp-none">No scenario or run matches “'+escH(q.value.trim())+'”.</div>';
  count.textContent=shown.length+' of '+S.traces.length;
  if(active>=shown.length)active=shown.length-1;
  if(active<0&&shown.length){const c=shown.findIndex(t=>cur&&t.id===cur.id);active=term||c<0?0:c;}
  mark();}
 function mark(){$$('.rp-opt',list).forEach(o=>o.classList.toggle('on',+o.dataset.i===active));const el=active>=0?$('#rp-o-'+active,list):null;
  if(el){q.setAttribute('aria-activedescendant',el.id);el.scrollIntoView({block:'nearest'});}else q.removeAttribute('aria-activedescendant');}
 function show(seed){if(open)return;open=true;pop.hidden=false;scrim.hidden=false;wrap.classList.add('open');btn.setAttribute('aria-expanded','true');
  q.value=seed||'';active=-1;draw();q.focus({preventScroll:true});}
 function hide(back){if(!open)return;open=false;pop.hidden=true;scrim.hidden=true;wrap.classList.remove('open');btn.setAttribute('aria-expanded','false');if(back)btn.focus();}
 function choose(i){const t=shown[i];if(!t)return;hide(true);load(t.id);ease(fitVB(),0);if(!reduce.matches)play();}
 btn.addEventListener('click',()=>open?hide(true):show());
 btn.addEventListener('keydown',e=>{if(e.key==='ArrowDown'||e.key==='ArrowUp'||e.key==='Enter'){e.preventDefault();e.stopPropagation();show();}
  else if(e.key.length===1&&e.key!==' '&&!e.ctrlKey&&!e.metaKey&&!e.altKey){e.preventDefault();e.stopPropagation();show(e.key);}});
 q.addEventListener('input',()=>{active=-1;draw();});
 q.addEventListener('keydown',e=>{const n=shown.length;
  if(e.key==='ArrowDown'){e.preventDefault();active=n?(active+1)%n:-1;mark();}
  else if(e.key==='ArrowUp'){e.preventDefault();active=n?(active-1+n)%n:-1;mark();}
  else if(e.key==='Home'&&!q.value){e.preventDefault();active=n?0:-1;mark();}
  else if(e.key==='End'&&!q.value){e.preventDefault();active=n-1;mark();}
  else if(e.key==='PageDown'){e.preventDefault();active=Math.min(n-1,active+8);mark();}
  else if(e.key==='PageUp'){e.preventDefault();active=Math.max(0,active-8);mark();}
  else if(e.key==='Enter'){e.preventDefault();choose(active);}
  else if(e.key==='Escape'){e.preventDefault();e.stopPropagation();if(q.value){q.value='';active=-1;draw();}else hide(true);}
  else if(e.key==='Tab')hide(false);
  e.stopPropagation();});
 list.addEventListener('mousedown',e=>e.preventDefault());
 list.addEventListener('click',e=>{const o=e.target.closest('.rp-opt');if(o)choose(+o.dataset.i);});
 list.addEventListener('pointermove',e=>{const o=e.target.closest('.rp-opt');if(o&&+o.dataset.i!==active){active=+o.dataset.i;$$('.rp-opt',list).forEach(x=>x.classList.toggle('on',x===o));q.setAttribute('aria-activedescendant',o.id);}});
 scrim.addEventListener('click',()=>hide(true));
 document.addEventListener('pointerdown',e=>{if(open&&!wrap.contains(e.target))hide(false);});
 return{set,show,hide,isOpen:()=>open};
})();

/* ---------- playback ---------- */
function slugSVG(e,t,a,b){return 'dm-'+slug(e)+'-'+slug(t)+'-'+slug(a)+'-'+slug(b);}
function caption(s,i){const c=$('#dm-caption');let h;
 if(!s)h=st.trace&&!st.trace.steps.length?'<span class="cap-err">This run has no steps: no command input could be built from values ESS gives. Generate with --suite to use its literals.</span>':'';
 else if(s.k==='execute_command')h='#'+(i+1)+' <b>'+escH(s.a?short(s.a):'(no actor)')+'</b> → '+escH((s.comp||[]).join(', ')||'no component')+' : <b>'+escH(short(s.c))+'</b> → '+
  (s.e?'<span class="cap-err">'+escH(short(s.e))+'</span>':s.o?'<span class="cap-ok">'+escH(s.o)+'</span>':'<span class="cap-err">no declared outcome</span>')+(s.note?' <span class="dim">'+escH(s.note)+'</span>':'');
 else h='#'+(i+1)+' '+escH(s.k)+': '+(s.u?'not evaluable':s.ok?'<span class="cap-ok">met</span>':s.ok===false?'<span class="cap-err">unmet</span>':'')+' <span class="dim">'+escH(s.t||'')+'</span>';
 c.innerHTML='<span class="cap-t">'+h+'</span>';c.title=c.textContent;}
function chrome(){const tr=st.trace,s=tr.steps[st.k-1];$('#dm-step').textContent=st.k;$('#dm-n').textContent=tr.steps.length;$('#dm-scrub').max=tr.steps.length;$('#dm-scrub').value=st.k;
 const state=stateAt(tr,st.k);$('#dm-time').textContent=state.at?state.at+' · ':'';
 $$('.dm-actor').forEach(b=>b.classList.toggle('acting',!!(s&&s.k==='execute_command'&&s.a===b.dataset.actor)));
 $$('.dm-col').forEach(c=>c.classList.toggle('lit',!!(s&&s.k==='execute_command'&&(s.comp||[]).includes(c.dataset.comp))));
 caption(s,st.k-1);renderDock();moveCursor();renderPass();if(window.essState)window.essState();}
function seek(k,ms){st.gen++;clearTimeout(st.timer);st.k=Math.max(0,Math.min(st.trace.steps.length,k));$$('.edge.fire,.dm-tl.fire',svg).forEach(x=>x.classList.remove('fire'));
 $$('.dm-frame.changed',svg).forEach(x=>x.classList.remove('changed'));$$('.tok.refused',tokL).forEach(t=>t.classList.remove('refused','shake'));
 renderTokens(stateAt(st.trace,st.k));chrome();if(st.follow)reframe(ms==null?300:ms);}
function tween(g,path,ms,done){if(!path||reduce.matches||!ms){done();return;}const L=path.getTotalLength(),t0=performance.now(),gen=st.gen;
 const tick=now=>{if(gen!==st.gen)return;const k=Math.min(1,(now-t0)/ms),e=k<.5?2*k*k:1-Math.pow(-2*k+2,2)/2,p=path.getPointAtLength(e*L);place(g,p.x,p.y);if(k<1)requestAnimationFrame(tick);else done();};requestAnimationFrame(tick);}
function forward(){const tr=st.trace;if(st.k>=tr.steps.length)return false;const s=tr.steps[st.k],prev=stateAt(tr,st.k);st.k++;st.gen++;
 $$('.edge.fire,.dm-tl.fire',svg).forEach(x=>x.classList.remove('fire'));$$('.dm-frame.changed',svg).forEach(x=>x.classList.remove('changed'));
 $$('.tok.refused',tokL).forEach(t=>t.classList.remove('refused','shake'));$$('.tok .err',tokL).forEach(t=>t.textContent='');
 const next=stateAt(tr,st.k);renderTokens(next);chrome();
 if(s.k!=='execute_command')return true;
 const ents=[];const dur=900/st.speed;
 (s.d||[]).forEach(([e,id,b,a,t])=>{if(!ents.includes(e))ents.push(e);const f=frameOf(e);if(f)f.classList.add('changed');
  const g=tokL.querySelector('[data-tok="'+CSS.escape(id)+'"]');if(!g)return;
  if(b==null&&!reduce.matches){g.animate([{opacity:0},{opacity:1}],{duration:dur});}
  if(t&&b!=null&&a!=null){const pid=slugSVG(e,t,b,a),path=svg.querySelector('[data-p="'+CSS.escape(pid)+'"]'),lab=svg.querySelector('[data-tl="'+CSS.escape(pid)+'"]');
   if(path)path.classList.add('fire');if(lab){lab.classList.add('fire');const old=lab.textContent;lab.textContent=t+' · '+short(s.c);setTimeout(()=>{lab.textContent=old;},1800/st.speed);}
   const end=g.getAttribute('transform');tween(g,path,dur,()=>g.setAttribute('transform',end));}});
 if(s.ref&&(s.e||!s.o)){const g=s.si&&tokL.querySelector('[data-tok="'+CSS.escape(s.si)+'"]');if(s.se&&!ents.includes(s.se))ents.push(s.se);
  if(g){g.classList.add('refused');if(!reduce.matches)g.classList.add('shake');g.querySelector('.err').textContent=s.e?short(s.e):'no declared outcome';}
  else{const f=s.se&&frameOf(s.se);if(f)f.classList.add('changed');}}
 cause(s,ents);const f0=ents.map(frameOf).find(Boolean);sparks(s,f0?svgScr(+f0.dataset.x+(+f0.dataset.w)/2,+f0.dataset.y+(+f0.dataset.h)/2):null);
 if(st.follow&&ents.length)focusFrame(ents[0]);void prev;return true;}
function dwell(){const s=st.trace.steps[st.k-1];return(s&&s.k==='execute_command'?1700:380)/st.speed;}
function loop(){const gen=st.gen;st.timer=setTimeout(()=>{if(!st.playing)return;if(!forward()){pause();return;}loop();},dwell());void gen;}
function play(){if(st.k>=st.trace.steps.length)seek(0);st.playing=true;$('#dm-play').textContent='Pause';loop();if(window.essState)window.essState();}
function pause(){st.playing=false;clearTimeout(st.timer);$('#dm-play').textContent='Play';if(window.essState)window.essState();}
function load(id,k){const tr=byId[id]||byId[S.demo]||S.traces[0];if(!tr)return;pause();st.trace=tr;st.sel=null;tokL.innerHTML='';picker.set(tr);
 renderLanes(tr);seek(k||0,0);}
function setSpeed(v){st.speed=+v||1;$$('.speedseg[data-a="dm-speed"] [data-speed]').forEach(b=>b.classList.toggle('on',+b.dataset.speed===st.speed));if(window.essState)window.essState();}
$$('.speedseg[data-a="dm-speed"] [data-speed]').forEach(b=>b.addEventListener('click',()=>{setSpeed(b.dataset.speed);if(window.essSpeed)window.essSpeed(st.speed);}));
root.addEventListener('click',e=>{const b=e.target.closest('[data-dm]');if(!b)return;const a=b.dataset.dm;
 if(a==='play')st.playing?pause():play();if(a==='next'){pause();forward();}if(a==='prev'){pause();seek(st.k-1);}if(a==='first'){pause();seek(0);}if(a==='fit')ease(fitVB(),400);});
$('#dm-scrub').addEventListener('input',e=>{pause();seek(+e.target.value);});
$('#dm-follow').addEventListener('change',e=>{st.follow=e.target.checked;if(st.follow)reframe(400);if(window.essState)window.essState();});
document.addEventListener('keydown',e=>{if(!document.body.classList.contains('tab-demo')||document.body.classList.contains('present'))return;
 if(e.target.closest&&e.target.closest('input,select,textarea,.rp'))return;
 if((e.key==='r'||e.key==='R')&&!e.ctrlKey&&!e.metaKey&&!e.altKey){e.preventDefault();picker.show();return;}
 if(e.key===' '){e.preventDefault();st.playing?pause():play();}if(e.key==='ArrowRight'){e.preventDefault();pause();forward();}if(e.key==='ArrowLeft'){e.preventDefault();pause();seek(st.k-1);}});
root.addEventListener('click',e=>{const a=e.target.closest('[data-model-ref]');if(a&&window.ESS){e.preventDefault();window.ESS.go(a.dataset.modelRef);}});
window.essDemo={load,seek,play,pause,setSpeed,forward,pick:()=>picker.show(),get:()=>({trace:st.trace&&st.trace.id,k:st.k,speed:st.speed,playing:st.playing,follow:st.follow}),
 setFollow:v=>{st.follow=!!v;$('#dm-follow').checked=st.follow;},fit:()=>ease(fitVB(),0),reframe:()=>reframe(0),traces:S.traces.map(t=>({id:t.id,title:t.title,group:t.group,origin:t.origin}))};
load(S.demo);requestAnimationFrame(()=>reframe(0));
if('ResizeObserver' in window){let lastW=0,lastH=0;new ResizeObserver(es=>{const r=es[0].contentRect;if(r.width>0&&r.height>0&&(Math.abs(r.width-lastW)>2||Math.abs(r.height-lastH)>2)){lastW=r.width;lastH=r.height;reframe(0);}}).observe(svg);}
else window.addEventListener('resize',()=>reframe(0));
window.essDemoAutoplay=()=>{if(!reduce.matches)play();};
})();
