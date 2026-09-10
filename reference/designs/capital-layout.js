// Pure seeded composition. Coordinates are in world pixels.
(function(root){
  const TILE_PX=24,R=TILE_PX/2*.94,APOTHEM=R*Math.sqrt(3)/2,INSET=.5;
  function hash(s){let n=2166136261;for(let i=0;i<s.length;i++)n=Math.imul(n^s.charCodeAt(i),16777619);return n>>>0;}
  function rng(s){let a=hash(s);return()=>{a=(a+0x6d2b79f5)>>>0;let t=Math.imul(a^(a>>>15),1|a);t^=t+Math.imul(t^(t>>>7),61|t);return ((t^(t>>>14))>>>0)/4294967296;};}
  function rectangle(x,z,w,d){return {x,z,w,d};}
  function bounds(b){return {x0:b.x-b.w/2,x1:b.x+b.w/2,z0:b.z-b.d/2,z1:b.z+b.d/2};}
  function inside(b,margin=INSET){const q=bounds(b);for(const x of [q.x0,q.x1])for(const z of [q.z0,q.z1])for(let i=0;i<6;i++){const a=Math.PI/6+i*Math.PI/3;if(x*Math.cos(a)+z*Math.sin(a)>APOTHEM-margin+1e-8)return false;}return true;}
  function overlap(a,b){return Math.min(a.x+a.w/2,b.x+b.w/2)-Math.max(a.x-a.w/2,b.x-b.w/2)>1e-8&&Math.min(a.z+a.d/2,b.z+b.d/2)-Math.max(a.z-a.d/2,b.z-b.d/2)>1e-8;}
  function validate(p){
    const errors=[];
    for(const b of [...p.buildings,...p.walls]){if(!inside(b))errors.push('outside hex');if(b.w<1||b.d<1||b.h<1)errors.push('invalid size');}
    for(let i=0;i<p.buildings.length;i++)for(let j=i+1;j<p.buildings.length;j++)if(overlap(p.buildings[i],p.buildings[j]))errors.push('building overlap');
    for(const w of p.walls)for(const b of p.buildings)if(overlap(w,b))errors.push('wall through building');
    for(const c of p.path)for(const b of [...p.buildings,...p.walls])if(overlap(c,b))errors.push('blocked entrance');
    for(const t of p.buildings)if(t!==p.keep&&t.h>=p.keep.h)errors.push('keep height');
    if(p.gate&&p.gate.gap!==3)errors.push('gate width');
    return [...new Set(errors)];
  }
  function generate(seedText,selection='mixed'){
    // Layout randomness is independent of paint, lighting, orientation and banner colour.
    const choose=rng(seedText+'|family'),family=selection==='mixed'?['A','B','C'][Math.floor(choose()*3)]:selection;
    for(let attempt=0;attempt<64;attempt++){
      const random=rng(seedText+'|'+family+'|'+attempt),pick=a=>a[Math.floor(random()*a.length)];
      const p={seed:seedText,family,buildings:[],walls:[],path:[],gate:null,keep:null,flag:null,angle:pick([0,1,2,3,4,5])*Math.PI/3,attempt};
      const building=(x,z,w,d,h,role)=>{const b={x,z,w,d,h,role};p.buildings.push(b);return b;};
      const wall=(x,z,w,d,h)=>{if(w>0&&d>0)p.walls.push({x,z,w,d,h});};
      const gateway=(x,z,span,h)=>{const gap=3,side=(span-gap)/2;p.gate={x,z,span,h,gap};wall(x-(gap+side)/2,z,side,1,h);wall(x+(gap+side)/2,z,side,1,h);};
      if(family==='A'){
        const w=pick([5,7]),d=pick([5,7]),h=pick([6,7,8]);p.keep=building(pick([-1,0,1]),pick([-2,-1,0]),w,d,h,'keep');
        if(random()<.72){const side=pick([-1,1]),ww=3,dd=pick([3,5]),z=p.keep.z-(d-Math.min(d,dd))/2;
          const wing={x:p.keep.x+side*(w+ww)/2,z,w:ww,d:Math.min(dd,d),h:pick([3,4]),role:'wing'};if(inside(wing))p.buildings.push(wing);
        }
      }else if(family==='B'){
        const dx=pick([3,4]),front=pick([3,4]),h=pick([4,5]),wallH=pick([2,3]);
        building(-dx,front,3,3,h,'gate tower');building(dx,front,3,3,h+pick([0,0,1]),'gate tower');
        p.keep=building(pick([-1,0,1]),pick([-3,-4]),5,pick([3,5]),h+pick([2,3]),'keep');gateway(0,front,2*dx-3,wallH);
        const enclosure=pick([0,1,2,2]),back=p.keep.z-p.keep.d/2-1;
        if(enclosure){const side=enclosure===1?pick([-1,1]):0,start=back+(enclosure===2?.5:0);for(const s of [-1,1])if(!side||side===s)wall(s*dx,(start+front-1.5)/2,1,front-1.5-start,wallH);if(enclosure===2)wall(0,back,2*dx+1,1,wallH);}
      }else{
        const dx=pick([4,5]),front=pick([4,5]),back=-pick([4,5]),base=pick([4,5]),watch=pick([0,1,2,3]);let i=0;
        for(const z of [front,back])for(const x of [-dx,dx])building(x,z,3,3,base+(i++===watch?1:0),'corner tower');
        const kw=pick([3,5]);p.keep=building(kw===3?pick([-1,0,1]):0,pick([-1,0]),kw,pick([3,5]),base+pick([2,3]),'keep');
        const wh=pick([2,3]);for(const x of [-dx,dx])wall(x,(back+front)/2,1,front-back-3,wh);wall(0,back,2*dx-3,1,wh);gateway(pick([-1,0,0,1]),front,2*dx-3,wh);
        // Shifted openings keep the front curtain anchored to its towers.
        if(p.gate.x!==0){p.walls.splice(-2);const g=p.gate,lo=-dx+1.5,hi=dx-1.5,lend=g.x-g.gap/2,rstart=g.x+g.gap/2;wall((lo+lend)/2,front,lend-lo,1,wh);wall((rstart+hi)/2,front,hi-rstart,1,wh);}
      }
      const doorZ=p.keep.z+p.keep.d/2,doorX=p.keep.x,gateX=p.gate?p.gate.x:doorX,gateZ=p.gate?p.gate.z:doorZ;
      const midZ=p.gate?(doorZ+gateZ-.5)/2:doorZ;
      if(p.gate){p.path.push(rectangle(doorX,(doorZ+midZ)/2,2,midZ-doorZ));if(doorX!==gateX)p.path.push(rectangle((doorX+gateX)/2,midZ,Math.abs(doorX-gateX)+2,2));p.path.push(rectangle(gateX,(midZ+9)/2,2,9-midZ));}
      else p.path.push(rectangle(doorX,(doorZ+9)/2,2,9-doorZ));
      const flagCandidates=p.buildings.filter(b=>b===p.keep||(family==='B'&&b.role==='gate tower'));
      const pole=pick(flagCandidates);p.flag={x:pole.x,z:pole.z,y:pole.h+1,poleHeight:pick([4,5]),direction:pick([-1,1])};
      if(!validate(p).length)return p;
    }
    throw new Error('No valid castle composition for this seed.');
  }

  const api={generate,validate,inside,hash};
  if(typeof module!=='undefined'&&module.exports)module.exports=api;else root.CapitalLayout=api;
})(globalThis);
