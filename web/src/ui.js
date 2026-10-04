(function(){
'use strict';
const essReplace=(s,t,u)=>{try{history.replaceState(s,t,u);}catch(e){}};
/* Tabs, URL state, zoom and pan, minimaps, collapsible sections, domain focus, command palette. */
const $=(s,r)=>(r||document).querySelector(s),$$=(s,r)=>Array.from((r||document).querySelectorAll(s));
const M=JSON.parse(document.getElementById('ess-model').textContent),D=M.decls;
const reduce=window.matchMedia('(prefers-reduced-motion: reduce)');
const escH=s=>String(s==null?'':s).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
const body=document.body;let restoring=false;

/* ---------- tabs ---------- */
const demoOff=body.classList.contains('demo-off');
function tab(name){if(demoOff)name='model';const was=body.classList.contains('tab-model')?'model':'demo';if(name===was&&body.classList.contains('tab-'+name))return;
 body.classList.toggle('tab-demo',name==='demo');body.classList.toggle('tab-model',name==='model');
 $$('.ab-tabs [data-tab]').forEach(b=>{b.classList.toggle('on',b.dataset.tab===name);b.setAttribute('aria-selected',b.dataset.tab===name);});
 if(name==='model'&&window.essDemo)window.essDemo.pause();
 if(name==='demo'&&window.essDemo)window.essDemo.reframe();
 if(body.classList.contains('present')&&window.ESS&&window.ESS.showSlide)window.ESS.showSlide(window.ESS.slide()-1);
 writeState();}
window.essTab=tab;
$$('.ab-tabs [data-tab]').forEach(b=>b.addEventListener('click',()=>tab(b.dataset.tab)));
$('#theme2').addEventListener('click',()=>$('#theme').click());

/* ---------- shared speed ---------- */
window.essSpeed=v=>{if(window.essDemo)window.essDemo.setSpeed(v);if(window.ESS&&window.ESS.setSpeed)window.ESS.setSpeed(v);writeState();};

/* ---------- domain focus ---------- */
let focus=null;
function setFocus(d){focus=d||null;body.classList.toggle('dfocus',!!focus);$$('.infocus').forEach(x=>x.classList.remove('infocus'));
 if(focus)$$('[data-dom]').forEach(x=>{if(x.dataset.dom.split(' ').includes(focus))x.classList.add('infocus');});
 $$('.dm-frame').forEach(f=>f.classList.toggle('infocus',!!focus&&f.dataset.dom===focus));
 const fb=$('#focusbar');if(fb)fb.querySelector('b').textContent=focus||'';writeState();}
window.essFocus=setFocus;
$$('.dlegend .dl').forEach(a=>{const d=a.textContent.trim();const b=document.createElement('span');b.setAttribute('role','button');b.tabIndex=0;b.className='dfocus-btn';b.textContent='focus';
 b.title='Dim everything outside '+d+' (Esc clears)';const act=e=>{e.preventDefault();e.stopPropagation();setFocus(focus===d?null:d);$$('.dl .dfocus-btn').forEach(x=>x.classList.toggle('on',focus!==null&&x===b));};
 b.addEventListener('click',act);b.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' ')act(e);});a.appendChild(b);});
const fb=document.createElement('span');fb.id='focusbar';fb.className='focusbar';fb.innerHTML='focus <b></b> <button type="button">clear</button>';
fb.querySelector('button').addEventListener('click',()=>setFocus(null));$('.ab-tools').prepend(fb);
document.addEventListener('keydown',e=>{if(e.key==='Escape'&&focus&&!body.classList.contains('present')&&$('#palette').hidden)setFocus(null);});

/* ---------- collapsible sections ---------- */
$$('main>section').forEach(s=>{const h=s.querySelector('h2');if(!h)return;const b=document.createElement('button');b.type='button';b.className='collapse';b.textContent='−';
 b.title='Collapse or expand this section';b.setAttribute('aria-expanded','true');
 b.addEventListener('click',()=>{const c=s.classList.toggle('collapsed');b.textContent=c?'+':'−';b.setAttribute('aria-expanded',String(!c));});h.appendChild(b);});

/* ---------- zoom and pan on large diagrams, with minimaps ---------- */
function zoomable(svg,withMap){const wrap=document.createElement('div');wrap.className='zwrap';svg.parentNode.insertBefore(wrap,svg);wrap.appendChild(svg);
 const full=svg.getAttribute('viewBox').split(/\s+/).map(Number);let vb=full.slice();
 const tools=document.createElement('div');tools.className='ztools';
 tools.innerHTML='<button type="button" data-z="in" title="Zoom in">+</button><button type="button" data-z="out" title="Zoom out">−</button><button type="button" data-z="reset" title="Reset">⟲</button>';wrap.appendChild(tools);
 let map=null,vp=null;
 if(withMap){map=document.createElement('div');map.className='minimap';const clone=svg.cloneNode(true);clone.removeAttribute('id');
  clone.querySelectorAll('[id]').forEach(x=>x.removeAttribute('id'));['data-ess-a','data-ess-id','data-ess-kind','data-ess-rel','data-dom','data-chg'].forEach(k=>clone.querySelectorAll('['+k+']').forEach(x=>x.removeAttribute(k)));
  clone.setAttribute('viewBox',full.join(' '));clone.removeAttribute('style');clone.classList.add('mmsvg');
  vp=document.createElementNS('http://www.w3.org/2000/svg','rect');vp.setAttribute('class','vp');clone.appendChild(vp);map.appendChild(clone);wrap.appendChild(map);
  map.addEventListener('click',e=>{const r=map.getBoundingClientRect(),x=full[0]+(e.clientX-r.left)/r.width*full[2],y=full[1]+(e.clientY-r.top)/r.height*full[3];
   set([x-vb[2]/2,y-vb[3]/2,vb[2],vb[3]]);});map.style.pointerEvents='auto';}
 function set(v){vb=v;svg.setAttribute('viewBox',v.map(n=>n.toFixed(1)).join(' '));const z=v[2]<full[2]-0.5;wrap.classList.toggle('zoomed',z);svg.classList.toggle('zoomed',z);
  if(vp){vp.setAttribute('x',v[0]);vp.setAttribute('y',v[1]);vp.setAttribute('width',v[2]);vp.setAttribute('height',v[3]);}}
 function zoom(f,cx,cy){const[x,y,w,h]=vb;cx=cx==null?x+w/2:cx;cy=cy==null?y+h/2:cy;const nw=Math.min(full[2],w*f),nh=Math.min(full[3],h*f),k=nw/w;
  set([cx-(cx-x)*k,cy-(cy-y)*(nh/h),nw,nh]);}
 tools.addEventListener('click',e=>{const b=e.target.closest('[data-z]');if(!b)return;if(b.dataset.z==='in')zoom(0.7);if(b.dataset.z==='out')zoom(1/0.7);if(b.dataset.z==='reset')set(full.slice());});
 svg.addEventListener('wheel',e=>{if(!(e.ctrlKey||e.metaKey))return;e.preventDefault();const pt=svg.createSVGPoint();pt.x=e.clientX;pt.y=e.clientY;const p=pt.matrixTransform(svg.getScreenCTM().inverse());
  zoom(Math.exp(e.deltaY*0.002),p.x,p.y);},{passive:false});
 const ptrs=new Map();let drag=null;
 svg.addEventListener('pointerdown',e=>{if(!wrap.classList.contains('zoomed')&&e.pointerType==='mouse')return;ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});drag={x:e.clientX,y:e.clientY,vb:vb.slice(),d:null,moved:false};});
 svg.addEventListener('pointermove',e=>{if(!ptrs.has(e.pointerId)||!drag)return;ptrs.set(e.pointerId,{x:e.clientX,y:e.clientY});const r=svg.getBoundingClientRect(),sc=vb[2]/(r.width||1);
  if(ptrs.size===2){const[a,b]=[...ptrs.values()];const d=Math.hypot(a.x-b.x,a.y-b.y);if(!drag.d){drag.d=d;drag.vb=vb.slice();return;}vb=drag.vb.slice();zoom(drag.d/d);return;}
  if(Math.abs(e.clientX-drag.x)+Math.abs(e.clientY-drag.y)>4){drag.moved=true;svg.setPointerCapture(e.pointerId);}
  if(drag.moved&&wrap.classList.contains('zoomed')){const[x,y,w,h]=drag.vb;set([x-(e.clientX-drag.x)*sc,y-(e.clientY-drag.y)*sc,w,h]);}});
 ['pointerup','pointercancel'].forEach(t=>svg.addEventListener(t,e=>{ptrs.delete(e.pointerId);if(!ptrs.size)drag=null;}));}
$$('svg.arch').forEach(s=>zoomable(s,true));$$('svg.er').forEach(s=>zoomable(s,true));$$('main svg.lifecycle[id]').forEach(s=>zoomable(s,false));

/* ---------- command palette ---------- */
const pal=$('#palette'),pq=$('#palette-q'),pl=$('#palette-list');let items=[],on=0;
function actions(){const a=[['action','Show the Demo',()=>tab('demo')],['action','Show the Model',()=>tab('model')],['action','Presentation mode',()=>window.ESS.enterPresent()],
 ['action','Toggle theme',()=>$('#theme').click()],['action','Play the Demo',()=>{tab('demo');window.essDemo.play();}],['action','Pause the Demo',()=>window.essDemo.pause()],
 ['action','Clear domain focus',()=>setFocus(null)]];
 Object.keys(M.decls).forEach(k=>{const d=D[k];if(d.k==='domain')a.push(['focus','Focus domain '+d.i,()=>setFocus(d.i)]);});
 (window.essDemo?window.essDemo.traces:[]).forEach(t=>a.push([t.group==='seeded runs'?'run':'scenario',(t.group==='seeded runs'?'Play ':'Open scenario ')+t.title,()=>{tab('demo');window.essDemo.load(t.id);if(!reduce.matches)window.essDemo.play();}]));
 Object.keys(D).sort().forEach(k=>{const d=D[k];a.push([d.k,d.i,()=>window.ESS.go(k)]);if(d.src)a.push(['source','Open source of '+d.i,()=>window.open(d.src[2],'_blank','noopener')]);});
 return demoOff?a.filter(x=>!/Demo/.test(x[1])&&x[0]!=='run'&&x[0]!=='scenario'):a;}
let all=null;
function draw(){const t=pq.value.trim().toLowerCase();all=all||actions();items=(t?all.filter(x=>(x[0]+' '+x[1]).toLowerCase().includes(t)):all).slice(0,60);on=Math.min(on,Math.max(0,items.length-1));
 pl.innerHTML=items.map((x,i)=>'<li data-i="'+i+'" class="'+(i===on?'on':'')+'"><span class="k">'+escH(x[0])+'</span><span>'+escH(x[1])+'</span></li>').join('')||'<li class="k">no match</li>';}
function openPal(){pal.hidden=false;pq.value='';on=0;draw();pq.focus();}
function closePal(){pal.hidden=true;}
function run(i){const x=items[i];closePal();if(x)x[2]();}
$('#palette-btn').addEventListener('click',openPal);
pq.addEventListener('input',()=>{on=0;draw();});
pq.addEventListener('keydown',e=>{if(e.key==='ArrowDown'){e.preventDefault();on=Math.min(items.length-1,on+1);draw();}if(e.key==='ArrowUp'){e.preventDefault();on=Math.max(0,on-1);draw();}
 if(e.key==='Enter'){e.preventDefault();run(on);}if(e.key==='Escape'){e.preventDefault();closePal();}});
pl.addEventListener('click',e=>{const li=e.target.closest('[data-i]');if(li)run(+li.dataset.i);});
document.addEventListener('keydown',e=>{if((e.ctrlKey||e.metaKey)&&(e.key==='k'||e.key==='K')){e.preventDefault();pal.hidden?openPal():closePal();}},true);

/* ---------- URL state: #!tab=…&trace=…&step=…&speed=…&follow=…&focus=…&q=…&slide=… ---------- */
let wtimer=0;
function writeState(){if(restoring)return;clearTimeout(wtimer);wtimer=setTimeout(()=>{const p=new URLSearchParams();p.set('tab',body.classList.contains('tab-model')?'model':'demo');
 const d=window.essDemo&&window.essDemo.get();if(d){p.set('trace',d.trace);p.set('step',d.k);if(d.speed!==1)p.set('speed',d.speed);if(!d.follow)p.set('follow','0');}
 if(focus)p.set('focus',focus);const q=$('#q');if(q&&q.value)p.set('q',q.value);
 if(body.classList.contains('present')){const n=$('#slidebar .n');if(n)p.set('slide',n.textContent.split('/')[0].trim());}
 const plain=location.hash&&!location.hash.startsWith('#!')?location.hash:null;
 if(plain&&p.get('tab')==='model')return;essReplace(null,'','#!'+p.toString());},250);}
window.essState=writeState;
$('#q').addEventListener('input',writeState);
function readState(){const h=location.hash;if(!h.startsWith('#!'))return false;const p=new URLSearchParams(h.slice(2));restoring=true;
 if(p.get('speed'))window.essSpeed(+p.get('speed'));
 if(window.essDemo){if(p.get('follow')==='0')window.essDemo.setFollow(false);if(p.get('trace')||p.get('step'))window.essDemo.load(p.get('trace')||null,+(p.get('step')||0));}
 tab(p.get('tab')==='model'?'model':'demo');
 if(p.get('focus'))setFocus(p.get('focus'));
 if(p.get('q')){tab('model');window.ESS.search(p.get('q'));}
 if(p.get('slide')){window.ESS.enterPresent();window.ESS.showSlide(+p.get('slide')-1);}
 restoring=false;return true;}

/* ---------- start ---------- */
const plain=decodeURIComponent(location.hash.slice(1));
if(new URLSearchParams(location.search).get('present')||body.classList.contains('present-auto')||body.classList.contains('present'))tab('model');
else if(plain&&!plain.startsWith('!')&&D[plain]){tab('model');window.ESS.go(plain);}
else if(!readState()){tab(demoOff||new URLSearchParams(location.search).get('tab')==='model'?'model':'demo');if(body.classList.contains('tab-demo')&&window.essDemoAutoplay)window.essDemoAutoplay();}
})();
