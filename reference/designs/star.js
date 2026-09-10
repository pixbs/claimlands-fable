// Availability token, distinct from the prototype background stars.
window.AvailabilityStar={
  radius:3,period:8,
  build(kit){
    const {T,poly}=kit,g=new T.Group(),STAR_RADIUS=3;
    // A carved five-point token: clipped tips, broad facets and a one-WP edge.
    const ring=[];for(let i=0;i<5;i++){
      const a=Math.PI/2+i*Math.PI*2/5,n=[Math.cos(a),Math.sin(a)],t=[-n[1],n[0]],b=a+Math.PI/5;
      ring.push([n[0]*STAR_RADIUS-t[0]*.45,n[1]*STAR_RADIUS-t[1]*.45],[n[0]*STAR_RADIUS+t[0]*.45,n[1]*STAR_RADIUS+t[1]*.45],[Math.cos(b)*1.55,Math.sin(b)*1.55]);
    }
    for(let i=0;i<ring.length;i++){
      const a=ring[i],b=ring[(i+1)%ring.length];
      poly([[0,0,1],[...a,.5],[...b,.5]],i%3===1?'#dbbd69':'#c5a557',g);
      poly([[0,0,-1],[...b,-.5],[...a,-.5]],i%3===1?'#c9aa59':'#b59448',g);
      poly([[...a,.5],[...a,-.5],[...b,-.5],[...b,.5]],'#9f7b38',g);
    }

    return g;
  },
  orient(star,camera,seconds){
    const spin=new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0,1,0),seconds*Math.PI*2/this.period);
    star.quaternion.copy(camera.quaternion).multiply(spin);
  }
};
