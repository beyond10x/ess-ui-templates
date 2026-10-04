(function(){
'use strict';
const essReplace=(s,t,u)=>{try{history.replaceState(s,t,u);}catch(e){}};
const M=JSON.parse(document.getElementById('ess-model').textContent);
const D=M.decls;
const reduce=window.matchMedia('(prefers-reduced-motion: reduce)');
const params=new URLSearchParams(location.search);if(document.body.classList.contains('present-auto')&&!params.has('present'))params.set('present','1');
const short=q=>String(q).split('.').pop();
const $=(s,r)=>(r||document).querySelector(s), $$=(s,r)=>Array.from((r||document).querySelectorAll(s));
const escH=s=>String(s).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));

/* ---------- theme ---------- */
const root=document.documentElement;

function label(){const b=$('#theme');if(b)b.textContent=root.dataset.theme==='dark'?'Light':'Dark';}
function setTheme(t){root.dataset.theme=t;try{localStorage.setItem('ess-theme',t);}catch(e){}label();}
label();
$('#theme').addEventListener('click',()=>setTheme(root.dataset.theme==='dark'?'light':'dark'));
window.addEventListener('message',e=>{const d=e.data;if(d&&d.type==='ess-ui:theme'&&(d.theme==='light'||d.theme==='dark')){root.dataset.theme=d.theme;label();}});

/* ---------- indexes: declaration anchor -> elements, and -> elements that relate to it ---------- */
const byA=new Map(), byRel=new Map();
$$('[data-ess-a]').forEach(el=>{const a=el.dataset.essA;if(!byA.has(a))byA.set(a,[]);byA.get(a).push(el);});
$$('[data-ess-rel]').forEach(el=>el.dataset.essRel.split(' ').forEach(a=>{if(!byRel.has(a))byRel.set(a,[]);byRel.get(a).push(el);}));
let hlOn=[];
function highlight(a){hlOn.forEach(el=>el.classList.remove('hl'));hlOn=[];if(!a)return;
 (byA.get(a)||[]).concat(byRel.get(a)||[]).forEach(el=>{el.classList.add('hl');hlOn.push(el);});
 const c=D[a];if(c&&c.k==='transition'){/* its causes too */}
}
document.addEventListener('pointerover',e=>{const el=e.target.closest('[data-ess-a]');highlight(el?el.dataset.essA:null);});

/* ---------- inspector ---------- */
const insp=$('#inspector');
function srcHtml(s){return s?'<a class="src" href="'+escH(s[2])+'" target="_blank" rel="noopener">'+escH(s[0])+':'+s[1]+'</a>':'<span class="dim">no authored line located</span>';}
function openInspector(a){const d=D[a];if(!d)return;
 let h='<button class="x" data-close aria-label="Close">×</button><div class="ik">'+escH(d.k)+'</div><h3>'+escH(d.t)+'</h3><div class="iid">'+escH(d.i)+'</div>';
 if(d.s)h+='<p class="summ">'+escH(d.s)+'</p>';
 h+='<dl><dt>source</dt><dd>'+srcHtml(d.src)+'</dd><dt>link</dt><dd><a href="#'+escH(a)+'" data-ref="'+escH(a)+'">#'+escH(a)+'</a></dd>';
 (d.f||[]).forEach(f=>{h+='<dt>'+escH(f[0])+'</dt><dd>'+escH(f[1])+'</dd>';});h+='</dl>';
 const groups={};(d.r||[]).forEach(r=>{(groups[r[0]]=groups[r[0]]||[]).push(r);});
 Object.keys(groups).forEach(g=>{h+='<div class="rg"><h4>'+escH(g)+'</h4>'+groups[g].map(r=>'<a class="chip ref" href="#'+escH(r[1])+'" data-ref="'+escH(r[1])+'" title="'+escH((D[r[1]]||{}).i||'')+'">'+escH(r[2])+'</a>').join(' ')+'</div>';});
 insp.innerHTML=h;insp.classList.add('open');insp.setAttribute('aria-hidden','false');}
function closeInspector(){insp.classList.remove('open');insp.setAttribute('aria-hidden','true');}
insp.addEventListener('click',e=>{if(e.target.closest('[data-close]'))closeInspector();});

/* ---------- deep links ---------- */
let lastTarget=[];
function markTarget(a,el){lastTarget.forEach(x=>x.classList.remove('target'));lastTarget=(byA.get(a)||[el]).slice();
 const r=el&&catRow.size?catSlideOf(el).row:null;if(r&&!lastTarget.includes(r))lastTarget.push(r);lastTarget.forEach(x=>x.classList.add('target'));}
function go(a,opts){opts=opts||{};if(window.essTab)window.essTab("model");const el=document.getElementById(a)||(byA.get(a)||[])[0];if(!el)return false;
 if(document.body.classList.contains('present')){const n=slideOf(el);if(n>=0){showSlide(n);markTarget(a,el);if(opts.inspect!==false)openInspector(a);return true;}exitPresent();}
 markTarget(a,el);
 el.scrollIntoView({block:'center',behavior:reduce.matches?'auto':'smooth'});
 if(opts.inspect!==false)openInspector(a);
 if(opts.push!==false&&location.hash!=='#'+a)essReplace(null,'','#'+a);return true;}
function fromHash(){const a=decodeURIComponent(location.hash.slice(1));if(a&&D[a])go(a,{push:false});}
window.addEventListener('hashchange',fromHash);

document.addEventListener('click',e=>{
 const cp=e.target.closest('button.copy');if(cp){e.preventDefault();const a=cp.dataset.copy;const url=location.href.split('#')[0]+'#'+a;
  const done=()=>{cp.textContent='✓';setTimeout(()=>cp.textContent='#',1200);};
  if(navigator.clipboard&&navigator.clipboard.writeText)navigator.clipboard.writeText(url).then(done,()=>{essReplace(null,'','#'+a);done();});
  else{essReplace(null,'','#'+a);done();}return;}
 const ref=e.target.closest('a.ref,[data-ref]');if(ref&&ref.dataset.ref){e.preventDefault();go(ref.dataset.ref);return;}
 if(e.target.closest('a,button,select,input,label,#inspector'))return;
 const el=e.target.closest('[data-ess-a]');if(el)openInspector(el.dataset.essA);});

/* ---------- search ---------- */
const q=$('#q'),qc=$('#qcount');let matches=[],mi=-1;
function search(){const t=q.value.trim().toLowerCase();$$('.match').forEach(el=>el.classList.remove('match','current'));matches=[];mi=-1;
 if(!t){document.body.classList.remove('searching');qc.textContent='';return;}
 document.body.classList.add('searching');const set=new Set();
 Object.keys(D).forEach(a=>{const d=D[a];if((d.i+' '+d.t).toLowerCase().includes(t))set.add(a);});
 set.forEach(a=>(byA.get(a)||[]).forEach(el=>{el.classList.add('match');let p=el.parentElement&&el.parentElement.closest('[data-ess-a],.card');
  while(p){p.classList.add('match');p=p.parentElement&&p.parentElement.closest('[data-ess-a],.card');}}));
 matches=Array.from(set).map(a=>document.getElementById(a)).filter(Boolean).sort((x,y)=>x.compareDocumentPosition(y)&Node.DOCUMENT_POSITION_FOLLOWING?-1:1);
 qc.textContent=set.size+' match'+(set.size===1?'':'es');}
q.addEventListener('input',search);
q.addEventListener('keydown',e=>{if(e.key==='Enter'&&matches.length){e.preventDefault();$$('.current').forEach(x=>x.classList.remove('current'));
  mi=(mi+1)%matches.length;const el=matches[mi];el.classList.add('current');el.scrollIntoView({block:'center',behavior:reduce.matches?'auto':'smooth'});
  qc.textContent=(mi+1)+' / '+matches.length;}
 if(e.key==='Escape'){q.value='';search();q.blur();}});

/* ---------- scroll spy ---------- */
const navLinks=new Map($$('nav a.sec').map(a=>[a.getAttribute('href').slice(1),a]));
if('IntersectionObserver' in window){const io=new IntersectionObserver(es=>{es.forEach(en=>{if(en.isIntersecting){navLinks.forEach(l=>l.classList.remove('active'));
 const l=navLinks.get(en.target.id);if(l)l.classList.add('active');}});},{rootMargin:'-35% 0px -60% 0px'});
 $$('main>section').forEach(s=>io.observe(s));}

/* ---------- lifecycle players ---------- */
function lit(ids,on){ids.forEach(id=>(byA.get(id)||[]).forEach(el=>{if(el.closest('.arch'))el.classList.toggle('lit',on);}));}
class Player{
 constructor(lc,panel){this.lc=lc;this.svg=document.getElementById(lc.svg);this.panel=panel;this.token=$('.token',this.svg);
  this.count={};lc.edges.forEach(e=>this.count[e.id]=0);this.gen=0;this.playing=false;this.lastLit=[];this.mode='tour';this.pos=0;this.speed=1;
  this.reset(true);}
 node(s){return this.svg.querySelector('[data-s="'+CSS.escape(s)+'"]');}
 path(id){return this.svg.querySelector('[data-p="'+CSS.escape(id)+'"]');}
 place(s){const n=this.node(s);if(!n)return;this.token.setAttribute('cx',n.dataset.cx);this.token.setAttribute('cy',n.dataset.cy);this.token.classList.add('on');if(this.cam)this.cam(+n.dataset.cx,+n.dataset.cy);}
 mark(s){$$('.state.active',this.svg).forEach(n=>n.classList.remove('active'));const n=this.node(s);if(n)n.classList.add('active','visited');this.state=s;}
 setMode(m){this.pause();this.mode=m;$$('.edge.onpath',this.svg).forEach(p=>p.classList.remove('onpath'));
  if(m!=='tour'){(this.lc.paths[+m]||{edges:[]}).edges.forEach(id=>{const p=this.path(id);if(p)p.classList.add('onpath');});}this.reset(true);}
 reset(first){this.gen++;clearTimeout(this.timer);$$('.state.visited',this.svg).forEach(n=>n.classList.remove('visited'));
  $$('.edge.firing,.elabel.firing',this.svg).forEach(n=>n.classList.remove('firing'));this.pos=0;
  const en=this.lc.entry;this.state=null;
  if(en&&!first&&!reduce.matches){this.walk(en.id,en.causes[0]||null,this.lc.initial,'(created)');return;}
  this.mark(this.lc.initial);this.place(this.lc.initial);this.show('(created)',en&&en.causes[0]);
  if(!first&&this.playing)this.timer=setTimeout(()=>this.step(),1100/this.speed);}
 show(tr,c){if(!this.panel)return;const qq=r=>this.panel.querySelector('[data-r="'+r+'"]');
  qq('state').textContent=this.state||'—';qq('transition').textContent=tr||'—';
  qq('command').textContent=c?short(c.command)+(c.display?' — '+c.display:''):'—';
  qq('outcome').textContent=c?c.outcome:'—';qq('condition').textContent=c?c.condition:'—';
  qq('actor').textContent=c?(c.actors.map(short).join(', ')||'no actor'):'—';
  qq('component').textContent=c?(c.components.join(', ')||'none'):'—';
  qq('emits').textContent=c?(c.emits.map(e=>short(e.event)+'('+e.fields.join(', ')+')').join(' ')||'nothing'):'—';
  const done=this.lc.edges.filter(e=>this.count[e.id]>0).length;qq('cov').textContent=done+' / '+this.lc.edges.length+' transitions walked';
  this.panel.querySelector('.bar i').style.width=(this.lc.edges.length?100*done/this.lc.edges.length:100)+'%';
  lit(this.lastLit,false);this.lastLit=c?c.actors.map(a=>M.anchor.actor[a]).concat(c.components.map(x=>M.anchor.component[x])).filter(Boolean):[];lit(this.lastLit,true);}
 walk(pid,cause,to,label){const path=this.path(pid);const g=++this.gen;
  $$('.edge.firing,.elabel.firing',this.svg).forEach(n=>n.classList.remove('firing'));
  if(path){path.classList.add('firing');const lab=this.svg.querySelector('.elabel[data-edge="'+CSS.escape(pid)+'"]');if(lab)lab.classList.add('firing');}
  const fin=()=>{if(g!==this.gen)return;this.mark(to);this.place(to);this.show(label,cause);if(this.playing)this.timer=setTimeout(()=>this.step(),1100/this.speed);};
  if(!path||reduce.matches){fin();return;}
  const L=path.getTotalLength(),dur=Math.max(900,Math.min(2200,L*6))/this.speed,t0=performance.now();this.token.classList.add('on');
  const tick=now=>{if(g!==this.gen)return;const k=Math.min(1,(now-t0)/dur),e=k<.5?2*k*k:1-Math.pow(-2*k+2,2)/2,p=path.getPointAtLength(e*L);
   this.token.setAttribute('cx',p.x);this.token.setAttribute('cy',p.y);if(this.cam)this.cam(p.x,p.y);if(k<1)requestAnimationFrame(tick);else fin();};
  requestAnimationFrame(tick);}
 nextEdge(){if(this.state===null)return null;
  if(this.mode!=='tour'){const p=this.lc.paths[+this.mode];if(!p||this.pos>=p.edges.length)return null;const id=p.edges[this.pos];return this.lc.edges.find(e=>e.id===id)||null;}
  const outs=this.lc.edges.filter(e=>e.from===this.state);if(!outs.length||this.lc.terminal.includes(this.state))return null;
  let best=outs[0];outs.forEach(e=>{if(this.count[e.id]<this.count[best.id])best=e;});return best;}
 atEnd(){return this.state!==null&&this.nextEdge()===null;}
 step(){clearTimeout(this.timer);if(this.state===null)return;const e=this.nextEdge();
  if(!e){this.gen++;if(this.playing)this.timer=setTimeout(()=>this.reset(false),1500/this.speed);else this.reset(false);return;}
  this.count[e.id]++;this.pos++;this.walk(e.id,e.causes[0]||null,e.to,e.transition);}
 play(){this.playing=true;this.step();}
 pause(){this.playing=false;this.gen++;clearTimeout(this.timer);if(this.state===null){this.mark(this.lc.initial);this.place(this.lc.initial);}}
}
const players=new Map();
M.lifecycles.forEach(lc=>{const panel=document.getElementById(lc.svg+'-panel');const p=new Player(lc,panel);players.set(lc.svg,p);
 if(!panel)return;const b=panel.querySelector('[data-a="play"]');p.btn=b;
 const setBtn=()=>{b.textContent=p.playing?'Pause':'Play';};
 b.addEventListener('click',()=>{if(p.playing)p.pause();else p.play();setBtn();});
 panel.querySelector('[data-a="step"]').addEventListener('click',()=>{p.pause();setBtn();p.step();});
 panel.querySelector('[data-a="reset"]').addEventListener('click',()=>{p.pause();setBtn();p.reset(true);});
 panel.querySelector('[data-a="path"]').addEventListener('change',e=>{p.setMode(e.target.value);setBtn();});
 panel.querySelectorAll('[data-a="speed"] [data-speed]').forEach(sb=>sb.addEventListener('click',()=>{p.speed=+sb.dataset.speed||1;panel.querySelectorAll('[data-a="speed"] [data-speed]').forEach(x=>x.classList.toggle('on',x===sb));if(window.essSpeed)window.essSpeed(p.speed);}));
 if(!reduce.matches&&!params.get('present')){p.play();setBtn();}});

/* ---------- presentation mode ----------
 Every slide is laid out on a fixed 1600×842 stage and the stage is scaled to the window, so pagination and slide numbers
 are the same on every screen. Text is never scaled to fit: HTML content is drawn at S, diagrams at no less than SMIN of their
 user units. Content that does not fit is split: cards by grid row, tables by row (header repeated), wide diagrams into
 windows of whole items, and a lifecycle taller than the stage scrolls with its token. */
const ST={W:1600,H:842,PX:44,PT:20,PB:18,CH:66,S:1.17,SMIN:0.95,SMAX:1.6,BAR:58,LIVE:250,GAP:14,PP:8};
const AW=ST.W-2*ST.PX,AH=ST.H-ST.PT-ST.PB-ST.CH,LW=AW/ST.S,LH=AH/ST.S;
const SKIP=new Set(['sources','coverage']),EARLY=['legend'];
let slides=[],si=0,cur=null,built=false,extra=null;
const R=el=>el.getBoundingClientRect();
const domCol=new Map();
function domColour(d){if(!domCol.size){$$('[data-dom]').forEach(el=>{const c=el.style.getPropertyValue('--dc').trim();if(c&&!domCol.has(el.dataset.dom))domCol.set(el.dataset.dom,c);});
  $$('h3.dom-h').forEach(h=>{const c=h.style.getPropertyValue('--dc').trim(),t=h.textContent.trim();if(c&&!domCol.has(t))domCol.set(t,c);});}
 return domCol.get(d)||'';}
function secInfo(s){const h=$('h2',s),ix=h&&$('.idx',h);return{id:s.id,idx:ix?ix.textContent.trim():'',title:s.dataset.title||(h?h.textContent.trim():'')};}
function groupsOf(items){const g=[],seen=new Set();const add=(label,colour)=>{if(label&&!seen.has(label)){seen.add(label);g.push({label,colour:colour||''});}};
 items.forEach(it=>{if(it.head)add(it.head.textContent.trim(),it.head.style.getPropertyValue('--dc').trim());
  else it.els.forEach(e=>{const d=e.dataset&&e.dataset.dom;if(d)add(d,domColour(d));});});return g;}

/* stage: move the host's children into chrome + body + scaled inner, and back */
function mount(host,kind){unmount();const style=host.getAttribute('style');host.classList.add('present-host','deck-stage','k-'+kind);
 const chrome=document.createElement('div');chrome.className='deck-chrome';const body=document.createElement('div');body.className='slide-body';
 const inner=document.createElement('div');inner.className='slide-fit';while(host.firstChild)inner.appendChild(host.firstChild);
 body.appendChild(inner);host.appendChild(chrome);host.appendChild(body);inner.style.width=(kind==='hero'?AW:LW)+'px';
 Object.assign(host.style,{width:ST.W+'px',height:ST.H+'px',left:'0px',top:'0px',transform:'none'});
 cur={host,kind,chrome,body,inner,style,restore:[],pg:[]};return cur;}
function unmount(){if(!cur)return;const c=cur;cur=null;c.restore.forEach(f=>f());c.pg.forEach(el=>el.classList.remove('pg-off'));
 if(c.demo){c.host.classList.remove('present-host');return;}
 while(c.inner.firstChild)c.host.insertBefore(c.inner.firstChild,c.chrome);c.chrome.remove();c.body.remove();
 c.host.classList.remove('present-host','deck-stage','k-'+c.kind);if(c.style==null)c.host.removeAttribute('style');else c.host.setAttribute('style',c.style);}
function place(){if(!cur||cur.demo)return;const vw=innerWidth,vh=innerHeight-ST.BAR,k=Math.min(vw/ST.W,vh/ST.H);
 Object.assign(cur.host.style,{left:((vw-ST.W*k)/2)+'px',top:((vh-ST.H*k)/2)+'px',transform:'scale('+k+')'});}
function show(pg,set){cur.pg=pg;pg.forEach(el=>el.classList.toggle('pg-off',!set.has(el)));}
function innerH(){return cur.inner.offsetHeight;}

/* flow sections: intro paragraphs, domain headings, grid rows, table rows, other blocks */
function flowUnits(){const kids=Array.from(cur.inner.children),intro=[],items=[],pg=[];let head=null,content=false;
 kids.forEach(c=>{pg.push(c);
  if(c.tagName==='H2')return;
  if(!content&&c.tagName==='P'){intro.push(c);return;}
  if(c.matches('h3.dom-h')){head=c;return;}
  content=true;
  if(c.matches('.grid')){let cols=parseInt(getComputedStyle(c).columnCount,10);
   if(cols>3&&Array.from(c.children).some(x=>R(x).height>LH*0.8)){c.dataset.deckCols='3';cols=3;}
   if(cols>1)Array.from(c.children).forEach(x=>{pg.push(x);items.push({els:[x],cont:[c],head,rep:[],h:(R(x).height+12)/cols});});
   else{const rows=[];Array.from(c.children).forEach(x=>{pg.push(x);const t=R(x).top;const r=rows.find(r=>Math.abs(r.t-t)<3);if(r)r.els.push(x);else rows.push({t,els:[x]});});
    rows.forEach(r=>items.push({els:r.els,cont:[c],head,rep:[]}));}}
  else if(c.tagName==='TABLE'&&c.rows.length>1){const trs=Array.from(c.rows),hdr=trs.filter(t=>!t.querySelector('td'));
   trs.filter(t=>!hdr.includes(t)).forEach(t=>{pg.push(t);items.push({els:[t],cont:[c],head,rep:hdr});});}
  else items.push({els:[c],cont:[],head,rep:[]});
  head=null;});
 return{intro,items,pg};}
function flowSet(u,page){const s=new Set();if(page.intro)u.intro.forEach(e=>s.add(e));
 page.items.forEach(it=>{it.els.forEach(e=>s.add(e));it.cont.forEach(e=>s.add(e));if(it.head)s.add(it.head);});return s;}
function buildFlow(host,sec){mount(host,'flow');const u=flowUnits();if(!u.items.length){unmount();return[];}
 u.pg.forEach(e=>e.classList.toggle('pg-off',e.tagName==='H2'));const top=R(cur.inner).top;
 u.items.forEach(it=>{it.top=Math.min(...it.els.map(e=>R(e).top));});
 const end=R(cur.inner).bottom;u.items.forEach((it,i)=>{if(it.h==null)it.h=(i+1<u.items.length?u.items[i+1].top:end)-it.top;});
 const pages=[];let p={items:[],intro:true},h=u.items[0].top-top;
 u.items.forEach(it=>{if(p.items.length&&h+it.h>LH){pages.push(p);p={items:[],intro:false};h=0;}p.items.push(it);h+=it.h;});pages.push(p);
 const fits=pg=>{show(u.pg,flowSet(u,pg));return innerH()<=LH+1;};
 for(let i=0;i<pages.length;i++){const pgi=pages[i];
  while(!fits(pgi)&&pgi.items.length>1){const it=pgi.items.pop();if(!pages[i+1])pages.push({items:[],intro:false});pages[i+1].items.unshift(it);}
  fits(pgi);const lim=Math.max(LH+1,innerH());
  while(pages[i+1]&&pages[i+1].items.length){pgi.items.push(pages[i+1].items.shift());show(u.pg,flowSet(u,pgi));if(innerH()>lim){pages[i+1].items.unshift(pgi.items.pop());break;}}
  if(pages[i+1]&&!pages[i+1].items.length)pages.splice(i+1,1);}
 unmount();
 return pages.map((pg,i)=>({kind:'flow',host,sec,part:i+1,parts:pages.length,pg:u.pg,set:flowSet(u,pg),items:pg.items,intro:pg.intro?u.intro:[],
  groups:groupsOf(pg.items)}));}

/* lifecycles: one slide per entity, the section intro on the first only */
function buildLc(host,sec){const blocks=$$('.lc-block',host),kids=Array.from(host.children),intro=kids.filter(k=>k.tagName==='P');
 return blocks.map((b,i)=>{const set=new Set([b]);if(!i)intro.forEach(e=>set.add(e));const h=$('h3',b),a=h&&$('a.ref',h);
  return{kind:'lc',host,sec,sub:b,part:i+1,parts:blocks.length,pg:kids,set,intro:i?[]:intro,groups:[{label:a?a.textContent.trim():(b.dataset.title||''),colour:h?h.style.getPropertyValue('--dc').trim():''}]};});}
function sizeLc(s){const svg=$('svg.lifecycle',s.sub);if(!svg)return;const panel=svg.closest('.panel'),wrap=$('.lc-wrap',s.sub),vb=svg.dataset.vb.split(' ').map(Number);
 const aW=LW-ST.LIVE-ST.GAP-2*ST.PP-2,aH=LH-(R(wrap).top-R(cur.inner).top)-2*ST.PP-4;
 const s0=Math.max(ST.SMIN,Math.min(ST.SMAX,aW*ST.S/vb[2],aH*ST.S/vb[3])),w=vb[2]*s0/ST.S,h=vb[3]*s0/ST.S;
 svg.setAttribute('viewBox',vb.join(' '));svg.style.width=w+'px';svg.style.height=h+'px';cur.restore.push(()=>{svg.style.width='';svg.style.height='';});
 const p=players.get(svg.id),liveW=LW-ST.GAP-(w+2*ST.PP+2);let wide=w>aW+1;
 if(wide&&liveW>=200){wrap.style.gridTemplateColumns=(w+2*ST.PP+2)+'px '+liveW+'px';wide=false;cur.restore.push(()=>{wrap.style.gridTemplateColumns='';});}
 if(h>aH+1||wide){panel.style.height=(Math.min(aH,h+(wide?18:0))+2*ST.PP)+'px';panel.style.overflow='auto';
  cur.restore.push(()=>{panel.style.height='';panel.style.overflow='';if(p)p.cam=null;});
  if(p)p.cam=(x,y)=>{const k=s0/ST.S;panel.scrollTop=(y-vb[1])*k+ST.PP-panel.clientHeight/2;panel.scrollLeft=(x-vb[0])*k+ST.PP-panel.clientWidth/2;};}
 const live=$('.panel.live',s.sub),ch=Math.max(panel.offsetHeight,live?live.offsetHeight:0),room=LH-(R(wrap).top-R(cur.inner).top)-ch;
 if(room>8){wrap.style.marginTop=(room/3)+'px';cur.restore.push(()=>{wrap.style.marginTop='';});}}

/* diagrams wider or taller than the stage: windows of whole items, read in rows */
function buildWin(host,sec){mount(host,'win');const kids=Array.from(cur.inner.children);cur.pg=kids;kids.forEach(k=>k.classList.toggle('pg-off',k.tagName==='H2'));
 const dk=kids.find(k=>Array.from(k.querySelectorAll('svg[viewBox]')).some(s=>!s.classList.contains('lifecycle')&&s.viewBox.baseVal.width>400));
 const svg=Array.from(dk.querySelectorAll('svg[viewBox]')).find(s=>s.viewBox.baseVal.width>400);
 const full=(svg.dataset.vb||svg.getAttribute('viewBox')).split(/[\s,]+/).map(Number);svg.dataset.vb=full.join(' ');
 const introH=R(dk).top-R(cur.inner).top,intro=kids.filter(k=>k!==dk&&k.tagName!=='H2');
 const it=Array.from(svg.children).filter(c=>/^(g|text)$/.test(c.tagName)&&!c.matches('.rel,.alink,.alabel,.flow,.token')).map(el=>{const b=el.getBBox();return{el,x:b.x,y:b.y,w:b.width,h:b.height};}).filter(b=>b.w>0&&b.h>0);
 it.sort((a,b)=>a.y-b.y);const rows=[];it.forEach(b=>{const r=rows.find(r=>Math.abs(r.y-b.y)<24);if(r)r.items.push(b);else rows.push({y:b.y,items:[b]});});
 const order=rows.flatMap(r=>r.items.sort((a,b)=>a.x-b.x)),pad=16,fullW=full[2]*ST.SMIN<=AW-24;
 const box=bs=>{const x0=Math.min(...bs.map(b=>b.x)),y0=Math.min(...bs.map(b=>b.y)),x1=Math.max(...bs.map(b=>b.x+b.w)),y1=Math.max(...bs.map(b=>b.y+b.h));
  return fullW?[full[0],y0-pad,full[2],y1-y0+2*pad]:[x0-pad,y0-pad,x1-x0+2*pad,y1-y0+2*pad];};
 const room=first=>[AW-24,AH-(first?introH*ST.S:0)-24];
 const fitsIn=(bx,first)=>{const[rw,rh]=room(first);return Math.min(rw/bx[2],rh/bx[3])>=ST.SMIN;};
 const wins=[];let left=order.slice();
 while(left.length){const first=!wins.length,w=[left[0]];let rest=left.slice(1);
  for(;;){let best=-1,cost=Infinity;rest.forEach((b,j)=>{const bx=box(w.concat([b]));if(!fitsIn(bx,first))return;const a=bx[2]*bx[3];if(a<cost-1){cost=a;best=j;}});
   if(best<0)break;w.push(rest[best]);rest.splice(best,1);}
  const bx=box(w);rest=rest.filter(b=>{const inside=b.x>=bx[0]&&b.y>=bx[1]&&b.x+b.w<=bx[0]+bx[2]&&b.y+b.h<=bx[1]+bx[3];if(inside)w.push(b);return!inside;});
  w.sort((a,b)=>order.indexOf(a)-order.indexOf(b));wins.push(w);left=rest;}
 unmount();
 return wins.map((ws,i)=>{const set=new Set([dk]);if(!i)intro.forEach(e=>set.add(e));const byK=new Map();
  ws.forEach(b=>{const a=b.el.dataset&&b.el.dataset.essA,d=a&&D[a];if(!d)return;if(!byK.has(d.k))byK.set(d.k,[]);byK.get(d.k).push(d);});
  const groups=[];byK.forEach((ds,k)=>{if(ds.length>4)groups.push({label:ds.length+' '+(/y$/.test(k)?k.slice(0,-1)+'ies':k+'s'),colour:''});
   else ds.forEach(d=>{const dm=(d.r||[]).filter(r=>r[0]==='domain');groups.push({label:d.t||short(d.i),colour:dm.length===1?domColour(dm[0][2]):''});});});
  return{kind:'win',host,sec,part:i+1,parts:wins.length,pg:kids,set,svg,vb:box(ws),items:ws,intro:i?[]:intro,first:!i,groups};});}
function sizeWin(s){const svg=s.svg,[rw,rh]=[AW-24,AH-(s.first?(R(svg.closest('.slide-fit>*')).top-R(cur.inner).top)*ST.S:0)-24],vb=s.vb;
 const s0=Math.max(ST.SMIN,Math.min(ST.SMAX,rw/vb[2],rh/vb[3]));svg.setAttribute('viewBox',vb.join(' '));
 svg.style.width=(vb[2]*s0/ST.S)+'px';svg.style.height=(vb[3]*s0/ST.S)+'px';
 cur.restore.push(()=>{svg.setAttribute('viewBox',svg.dataset.vb);svg.style.width='';svg.style.height='';});}

/* compact forms. A reference section whose full form needs more than CAT.AT slides is presented as a catalogue instead:
 its declarations as rows (name, one key fact) grouped by domain, whole domains packed onto as few slides as fit.
 A row opens the inspector with the full card; the cards stay on the page. A table-shaped change list longer than
 one slide becomes one summary slide. Anything inside a compacted section resolves to its catalogue slide. */
const CAT={REF:new Set(['entities','commands','events','views','interactions','errors']),AT:6,CHUNK:24,KINDS:new Set(['entity','command','event','view','error','type','binding','workload']),ADDED:40};
const catRow=new Map(),catHost=new Map();
const plural=(n,w)=>n+' '+(n===1?w:(/y$/.test(w)?w.slice(0,-1)+'ies':w+'s'));
function catFact(d){const nr=k=>(d.r||[]).filter(r=>r[0]===k).length,nf=k=>(d.f||[]).filter(x=>x[0]===k).length,fv=k=>((d.f||[]).find(x=>x[0]===k)||[])[1];
 switch(d.k){
  case'command':return plural(nr('outcome'),'outcome')+(nr('event')?' · '+plural(nr('event'),'event'):'');
  case'entity':return plural(nf('field'),'field')+' · '+plural(nr('state'),'state');
  case'event':return plural(nf('field'),'field')+' · '+plural(nr('outcome'),'emitter');
  case'view':{const e=(d.r||[]).find(r=>r[0]==='entity');return e?'of '+e[2]:'';}
  case'error':return 'raised by '+plural(nr('outcome'),'outcome');
  case'type':{const k=fv('kind')||'',v=fv('variants');return k+(v?' · '+plural(v.split(',').length,'variant'):fv('of')?' of '+fv('of'):'');}
  default:return(d.f||[])[0]?d.f[0][0]+' '+d.f[0][1]:'';}}
function catDecls(sec){const out=[];$$('[data-ess-a][id]',sec).forEach(el=>{const a=el.dataset.essA,d=D[a];if(!d||!CAT.KINDS.has(d.k))return;
  let p=el.parentElement&&el.parentElement.closest('[data-ess-a][id]');while(p&&sec.contains(p)){const pd=D[p.dataset.essA];if(pd&&CAT.KINDS.has(pd.k))return;p=p.parentElement&&p.parentElement.closest('[data-ess-a][id]');}
  out.push({a,d,dom:(el.dataset.dom||'').split(' ')[0],chg:el.dataset.chg||''});});return out;}
function catSection(s,sec,lede){const el=document.createElement('section');el.className='deck-slide deck-cat';el.dataset.title=sec.title;el.dataset.catFor=s.id;
 el.innerHTML='<h2>'+escH(sec.title)+'</h2>'+(lede?'<p class="lede">'+lede+'</p>':'');deckExtra().appendChild(el);catHost.set(el,s);return el;}
function firstSentence(s){const l=$(':scope>p.lede',s);return l?l.textContent.trim().split(/(?<=\.)\s/)[0]:'';}
function buildCat(s,sec){const ds=catDecls(s);if(!ds.length)return[];const doms=[],by=new Map();
 ds.forEach(x=>{if(!by.has(x.dom)){by.set(x.dom,[]);doms.push(x.dom);}by.get(x.dom).push(x);});
 const kinds=new Set(ds.map(x=>x.d.k)),kc=[...kinds].map(k=>plural(ds.filter(x=>x.d.k===k).length,k)).join(', ');
 const lede=escH(kc)+' in '+plural(doms.filter(Boolean).length,'domain')+'. <span class="cat-note">Click one to inspect it; the full cards are on the page.</span>';
 const host=catSection(s,sec,lede),grid=document.createElement('div');grid.className='grid cat-grid';
 const row=x=>{const f=catFact(x.d);return'<span class="cat-row k-'+escH(x.d.k)+'" data-ess-a="'+escH(x.a)+'" tabindex="0" role="button" title="'+escH(x.d.i+(x.d.s&&x.d.s!==x.d.t?' — '+x.d.s:''))+'">'+
  '<span class="cr-n">'+escH(x.d.t||short(x.d.i))+(x.chg?'<span class="chgbadge '+escH(x.chg)+'">'+escH(x.chg)+'</span>':'')+'</span>'+(f?'<span class="cr-f">'+escH(f)+'</span>':'')+'</span>';};
 doms.forEach(dm=>{const xs=by.get(dm).slice().sort((p,q)=>kinds.size>1?[...kinds].indexOf(p.d.k)-[...kinds].indexOf(q.d.k):0),n=Math.ceil(xs.length/CAT.CHUNK),sz=Math.ceil(xs.length/n),c=dm?domColour(dm):'';
  for(let i=0;i<n;i++){const part=xs.slice(i*sz,(i+1)*sz),b=document.createElement('div');b.className='cat-dom';if(dm)b.dataset.dom=dm;if(c)b.style.setProperty('--dc',c);
   let h='<h4><i class="dsw"></i><span>'+escH(dm||'no domain')+'</span><b>'+(n>1?(i+1)+'/'+n+' · ':'')+xs.length+'</b></h4>',k0='';
   part.forEach(x=>{if(kinds.size>1&&x.d.k!==k0){k0=x.d.k;h+='<span class="cr-k">'+escH(k0)+'s</span>';}h+=row(x);});b.innerHTML=h;grid.appendChild(b);}});
 host.appendChild(grid);$$('.cat-row',grid).forEach(r=>{const a=r.dataset.essA;if(!byA.has(a))byA.set(a,[]);byA.get(a).push(r);catRow.set(a,r);});
 return buildFlow(host,sec).map(x=>Object.assign(x,{cat:s}));}
/* a change table longer than a slide: one summary slide, counts by category and kind, and what was added */
function buildChangeSummary(s,sec){const t=$('table',s);if(!t)return[];const th=Array.from(t.rows[0].cells).map(c=>c.textContent.trim());
 const ci=th.indexOf('category'),di=th.indexOf('declaration'),ki=th.indexOf('change');if(ci<0||ki<0)return[];
 const rows=Array.from(t.rows).slice(1).map(r=>({cat:r.cells[ci].textContent.trim(),kind:r.cells[ki].textContent.trim(),cell:di>=0?r.cells[di]:null}));
 const cls=k=>k==='added'?'added':k==='removed'?'removed':'changed',cats=[],by={};
 rows.forEach(r=>{if(!by[r.cat]){by[r.cat]={added:0,removed:0,changed:0,kinds:{}};cats.push(r.cat);}const b=by[r.cat];b[cls(r.kind)]++;if(cls(r.kind)==='changed')b.kinds[r.kind]=(b.kinds[r.kind]||0)+1;});
 const tot=k=>rows.filter(r=>cls(r.kind)===k).length;cats.sort((p,q)=>(rows.filter(r=>r.cat===q).length-rows.filter(r=>r.cat===p).length)||cats.indexOf(p)-cats.indexOf(q));
 const lede=escH(firstSentence(s))+' <span class="cat-note">One summary slide; every change is listed on the page and badged on its diagram and card.</span>';
 const host=catSection(s,sec,lede);let h='<div class="chg-stats">'+[['changes',rows.length,''],['added',tot('added'),'added'],['changed',tot('changed'),'changed'],['removed',tot('removed'),'removed']].map(x=>'<div class="stat '+x[2]+'"><b>'+x[1]+'</b>'+x[0]+'</div>').join('')+'</div>';
 h+='<table class="chg-sum"><tr><th>category</th><th>added</th><th>changed</th><th>removed</th><th>most frequent change</th></tr>'+cats.map(c=>{const b=by[c],top=Object.keys(b.kinds).sort((p,q)=>b.kinds[q]-b.kinds[p]||(p<q?-1:1)).slice(0,3).map(k=>k+' ×'+b.kinds[k]).join(', ');
  return'<tr><td class="mono">'+escH(c)+'</td><td class="n">'+(b.added||'')+'</td><td class="n">'+(b.changed||'')+'</td><td class="n">'+(b.removed||'')+'</td><td class="mono dim">'+escH(top)+'</td></tr>';}).join('')+'</table>';
 const add=rows.filter(r=>r.kind==='added'&&r.cell);if(add.length){const order=['domain','entity','command','event','view','error','type'],rk=c=>{const i=order.indexOf(c);return i<0?order.length:i;};
  add.sort((p,q)=>rk(p.cat)-rk(q.cat));const shown=add.slice(0,CAT.ADDED);
  h+='<h4 class="chg-h">added</h4><div class="chg-add">'+shown.map(r=>{const l=$('[data-ref]',r.cell),a=l&&l.dataset.ref,nm=r.cell.textContent.trim();
   return'<span class="chip'+(a&&D[a]?'" data-ess-a="'+escH(a)+'" tabindex="0" role="button':'')+'" title="'+escH(r.cat+' '+nm)+'"><i>'+escH(r.cat)+'</i> '+escH(a&&D[a]?D[a].t||short(nm):short(nm))+'</span>';}).join(' ')+
   (add.length>shown.length?' <span class="dim">+'+(add.length-shown.length)+' more on the page</span>':'')+'</div>';}
 host.insertAdjacentHTML('beforeend',h);return buildFlow(host,sec).map(x=>Object.assign(x,{cat:s}));}
/* the slide a compacted element resolves to: its declaration's row, else the section's first catalogue slide */
function catAnchor(el,sec){for(let x=el;x&&x!==sec;x=x.parentElement){const a=x.dataset&&x.dataset.essA;if(a&&catRow.has(a))return a;}
 const r=el.closest&&el.closest('[data-ess-rel]');if(r&&sec.contains(r))for(const a of r.dataset.essRel.split(' '))if(catRow.has(a))return a;return null;}
function catSlideOf(el){const sec=el.closest&&el.closest('main>section');if(!sec)return{n:-1};let first=-1;
 for(let i=0;i<slides.length;i++){if(slides[i].cat!==sec)continue;if(first<0)first=i;}if(first<0)return{n:-1};
 const a=catAnchor(el,sec);if(a){const r=catRow.get(a);for(let i=first;i<slides.length&&slides[i].cat===sec;i++)if(slides[i].items.some(it=>it.els.some(x=>x.contains(r))))return{n:i,row:r};}
 return{n:first};}
document.addEventListener('keydown',e=>{if((e.key==='Enter'||e.key===' ')&&e.target.matches&&e.target.matches('.deck-cat [data-ess-a][tabindex]')){e.preventDefault();e.stopPropagation();openInspector(e.target.dataset.essA);}},true);

/* opener, agenda */
function deckExtra(){if(extra)return extra;const hero=$('header.hero');
 hero.insertAdjacentHTML('beforeend','<div class="deck-howto"><h4>How to use this</h4><div><kbd>→</kbd><kbd>Space</kbd>next slide</div><div><kbd>←</kbd>previous slide</div>'+
  '<div><kbd>↓</kbd>step a lifecycle token</div><div><kbd>click</kbd>anything to inspect it</div><div><kbd>/</kbd>search, jump to its slide</div><div><kbd>a</kbd>agenda · <kbd>Esc</kbd>leave</div></div>');
 extra=document.createElement('div');extra.id='deck-extra';extra.innerHTML='<section class="deck-slide" id="deck-agenda" data-title="Agenda"><h2>Agenda</h2><p class="lede"></p><div class="grid agenda-grid"></div></section>';
 document.body.appendChild(extra);extra.addEventListener('click',e=>{const b=e.target.closest('[data-go]');if(b){e.preventDefault();showSlide(+b.dataset.go-1);}});return extra;}
function buildSlides(){deckExtra();const secs=$$('main>section').filter(s=>!SKIP.has(s.id));
 const ordered=EARLY.map(id=>secs.find(s=>s.id===id)).filter(Boolean).concat(secs.filter(s=>!EARLY.includes(s.id)));
 const rest=[];const demo=document.getElementById('demo');
 if(demo&&!document.body.classList.contains('demo-off'))rest.push({kind:'demo',host:demo,sec:{id:'demo',idx:'',title:'Demo'},part:1,parts:1,groups:[]});
 ordered.forEach(s=>{const sec=secInfo(s);if(EARLY.includes(s.id))sec.idx='';let out;
  if($('.lc-block',s))out=buildLc(s,sec);
  else if(Array.from(s.querySelectorAll('svg[viewBox]')).some(v=>!v.classList.contains('lifecycle')&&v.viewBox.baseVal.width>400))out=buildWin(s,sec);
  else out=buildFlow(s,sec);
  if(CAT.REF.has(s.id)&&out.length>CAT.AT){const c=buildCat(s,sec);if(c.length&&c.length<out.length)out=c;}
  else if(s.id==='changes'&&out.length>1){const c=buildChangeSummary(s,sec);if(c.length===1)out=c;}
  rest.push(...out);});
 const ag=$('#deck-agenda'),grid=$('.agenda-grid',ag),secs2=[];rest.forEach(s=>{if(s.part===1)secs2.push(s);});
 $('.lede',ag).textContent=secs2.length+' sections · '+rest.length+' slides. Click a section to jump to it.';
 grid.innerHTML=secs2.map(s=>{const lede=s.host.querySelector(':scope>p.lede'),d=lede?lede.textContent.trim().split(/(?<=\.)\s/)[0]:'';
  return '<button type="button" class="ag" data-go="0" data-sec="'+escH(s.sec.id)+'"><span class="ag-i">'+escH(s.sec.idx||'·')+'</span><span class="ag-t">'+escH(s.sec.title)+'</span><span class="ag-n"></span>'+
   (d?'<span class="ag-d">'+escH(d.length>150?d.slice(0,148)+'…':d)+'</span>':'')+'</button>';}).join('');
 const hero={kind:'hero',host:$('header.hero'),sec:{id:'hero',idx:'',title:($('header.hero h1')||{}).textContent||''},part:1,parts:1,groups:[]};
 const agenda=buildFlow(ag,{id:'agenda',idx:'',title:'Agenda'});slides=[hero].concat(agenda,rest);
 $('.lede',ag).textContent=secs2.length+' sections on '+slides.length+' slides. Click a section to jump to it; a returns here.';
 const at=new Map();slides.forEach((s,i)=>{if(!at.has(s.sec.id))at.set(s.sec.id,{first:i+1,n:0});at.get(s.sec.id).n++;});
 $$('.ag',grid).forEach(b=>{const r=at.get(b.dataset.sec);b.dataset.go=r.first;$('.ag-n',b).textContent=r.n>1?'slides '+r.first+'–'+(r.first+r.n-1):'slide '+r.first;});}

function chromeHtml(s){if(s.kind==='hero')return'';const pct=100*(si+1)/slides.length;
 return '<div class="dc-l"><span class="dc-i">'+escH(s.sec.idx)+'</span><span class="dc-t">'+escH(s.sec.title)+'</span>'+(s.parts>1?'<span class="dc-p">'+s.part+' / '+s.parts+'</span>':'')+
  (s.groups.length>4?s.groups.slice(0,3).concat([{label:'+'+(s.groups.length-3)+' more',colour:''}]):s.groups).map(g=>'<span class="dc-g"'+(g.colour?' style="--dc:'+escH(g.colour)+'"':'')+'><i class="dsw"></i>'+escH(g.label)+'</span>').join('')+'</div>'+
  '<div class="dc-r">'+(si+1)+' / '+slides.length+'</div><div class="dc-bar"><i style="width:'+pct.toFixed(2)+'%"></i></div>';}
function slideTitle(s){const g=s.groups.map(x=>x.label).slice(0,3).join(', ');return s.sec.title+(g?' · '+g:'')+(s.parts>1?' ('+s.part+'/'+s.parts+')':'');}
function showSlide(i){if(!slides.length)return;si=Math.max(0,Math.min(slides.length-1,i|0));unmount();$$('.present-host').forEach(x=>x.classList.remove('present-host'));
 const s=slides[si];
 if(s.kind==='demo'){s.host.classList.add('present-host');cur={host:s.host,demo:true,restore:[],pg:[]};if(window.essDemo){window.essDemo.pause();requestAnimationFrame(()=>window.essDemo.fit());}}
 else{mount(s.host,s.kind);cur.chrome.innerHTML=chromeHtml(s);const c=(s.groups[0]||{}).colour;if(c)cur.host.style.setProperty('--dc',c);
  if(s.pg)show(s.pg,s.set);s.host.scrollTop=0;
  if(s.kind==='lc'){sizeLc(s);const svg=$('svg.lifecycle',s.sub),p=svg&&players.get(svg.id);if(p){p.pause();if(p.btn)p.btn.textContent='Play';p.reset(true);}}
  if(s.kind==='win')sizeWin(s);
  if(s.kind!=='hero')cur.inner.style.transform='scale('+ST.S+')';place();}
 $('#slidebar .n').textContent=(si+1)+' / '+slides.length;$('#slidebar .t').textContent=slideTitle(s);
 $('#slidebar .sb-prog b').style.width=(100*(si+1)/slides.length).toFixed(2)+'%';
 $('#slidebar .hint').textContent=s.kind==='demo'?'↓ step the demo':s.kind==='lc'?'↓ step the token · click to inspect':'click to inspect';
 if(window.essState)window.essState();}
function slideOf(el){for(let i=0;i<slides.length;i++){const s=slides[i];if(!s.host.contains(el))continue;
 if(s.kind==='hero'||s.kind==='demo')return i;
 if((s.intro||[]).some(x=>x.contains(el)))return i;
 if(s.kind==='lc'&&s.sub.contains(el))return i;
 if(s.kind==='flow'&&s.items.some(it=>it.els.some(x=>x.contains(el))))return i;
 if(s.kind==='win'&&s.svg.contains(el)){if(s.items.some(b=>b.el.contains(el)))return i;
  try{const b=el.getBBox(),v=s.vb;if(b.x<v[0]+v[2]&&b.x+b.width>v[0]&&b.y<v[1]+v[3]&&b.y+b.height>v[1])return i;}catch(e){}}}
 return catSlideOf(el).n;}
window.addEventListener('resize',place);
function currentPlayer(){const s=slides[si];if(!s||s.kind!=='lc')return null;const svg=$('svg.lifecycle',s.sub);return svg&&players.get(svg.id);}
function enterPresent(){if(document.body.classList.contains('present'))return;if(window.essTab)window.essTab("model");closeInspector();
 players.forEach(p=>{p.pause();if(p.btn)p.btn.textContent='Play';});document.body.classList.add('present');
 if(!built){buildSlides();built=true;}showSlide(+(params.get('slide')||1)-1);if(!params.get('slide'))fromHash();
 if(document.documentElement.requestFullscreen&&!params.get('present'))document.documentElement.requestFullscreen().catch(()=>{});}
function exitPresent(){unmount();document.body.classList.remove('present');$$('.present-host').forEach(x=>x.classList.remove('present-host'));
 if(document.fullscreenElement&&document.exitFullscreen)document.exitFullscreen().catch(()=>{});}
$('#present').addEventListener('click',()=>{document.body.classList.contains('present')?exitPresent():enterPresent();});
const sbar=$('#slidebar');sbar.insertAdjacentHTML('afterbegin','<i class="sb-prog"><b></b></i>');
$('#slidebar .end').insertAdjacentHTML('beforebegin','<button class="agenda" type="button" title="Agenda (a)">Agenda</button><input class="sq" type="search" placeholder="Search  /" aria-label="Search declarations and jump to their slide"><span class="sqc"></span>');
$('#slidebar .prev').addEventListener('click',()=>showSlide(si-1));$('#slidebar .next').addEventListener('click',()=>forward());
$('#slidebar .end').addEventListener('click',exitPresent);$('#slidebar .agenda').addEventListener('click',()=>showSlide(1));
const sq=$('#slidebar .sq'),sqc=$('#slidebar .sqc');let hits=[],hk=-1;
sq.addEventListener('input',()=>{const t=sq.value.trim().toLowerCase();hits=[];hk=-1;if(!t){sqc.textContent='';return;}
 Object.keys(D).forEach(a=>{const d=D[a];if(!(d.i+' '+d.t).toLowerCase().includes(t))return;const el=document.getElementById(a)||(byA.get(a)||[])[0];const n=el?slideOf(el):-1;if(n>=0)hits.push({a,el,n});});
 hits.sort((x,y)=>x.n-y.n);sqc.textContent=hits.length?hits.length+' found · Enter':'none on a slide';});
sq.addEventListener('keydown',e=>{if(e.key==='Enter'&&hits.length){e.preventDefault();hk=(hk+1)%hits.length;const h=hits[hk];showSlide(h.n);markTarget(h.a,h.el);sqc.textContent=(hk+1)+' / '+hits.length;}
 if(e.key==='Escape'){e.preventDefault();sq.value='';sqc.textContent='';hits=[];sq.blur();}});
function forward(){showSlide(si+1);}
function stepInSlide(){const s=slides[si];if(s&&s.kind==='demo'&&window.essDemo){window.essDemo.pause();window.essDemo.forward();return;}
 const p=currentPlayer();if(p){p.pause();if(p.btn)p.btn.textContent='Play';p.step();}}
document.addEventListener('keydown',e=>{const typing=e.target.closest&&e.target.closest('input,select,textarea');
 if(typing)return;const on=document.body.classList.contains('present');
 if(e.key==='/'){e.preventDefault();(on?sq:q).focus();return;}
 if(e.key==='p'||e.key==='P'){e.preventDefault();on?exitPresent():enterPresent();return;}
 if(e.key==='Escape'){if(insp.classList.contains('open'))closeInspector();else if(on)exitPresent();return;}
 if(!on)return;
 if(e.key==='ArrowRight'||e.key===' '||e.key==='PageDown'){e.preventDefault();forward();}
 if(e.key==='ArrowLeft'||e.key==='PageUp'){e.preventDefault();showSlide(si-1);}
 if(e.key==='Home'){e.preventDefault();showSlide(0);}if(e.key==='End'){e.preventDefault();showSlide(slides.length-1);}
 if(e.key==='a'||e.key==='A'){e.preventDefault();showSlide(1);}
 if(e.key==='ArrowDown'||e.key==='.'){e.preventDefault();stepInSlide();}});

$$('svg.lifecycle[id]').forEach(s=>{s.dataset.vb=s.getAttribute('viewBox').trim().split(/[\s,]+/).join(' ');});
if(params.get('present'))window.addEventListener('DOMContentLoaded',enterPresent);
window.ESS={go,openInspector,closeInspector,enterPresent,exitPresent,showSlide:i=>showSlide(i),slide:()=>si+1,slides:()=>slides.length,players,highlight,search:t=>{q.value=t;search();},setSpeed:v=>players.forEach(p=>{p.speed=v;const pn=document.getElementById(p.lc.svg+'-panel');if(pn)pn.querySelectorAll('[data-speed]').forEach(x=>x.classList.toggle('on',+x.dataset.speed===v));})};
fromHash();
})();
