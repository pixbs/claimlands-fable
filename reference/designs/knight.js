// Approved original marker at local origin, in world pixels.
window.ReferenceModel=function(kit){
  const {T,P,DECAL,box,tapered,cutBox,flat}=kit,g=new T.Group();
  cutBox(0,0,0,5.6,1,4.2,1,P.dark,g);
  tapered(0,1,0,4.2,3.5,2.8,2.8,3.2,P.faction,g);
  cutBox(0,4.2,0,2.8,2.8,2.8,.6,P.metal,g);
  box(0,5.2,1.4+DECAL,2.8,1,DECAL,P.dark,g);
  box(0,7,-.2,1,1,1.8,P.faction,g);
  box(2.1,1,0,1,8.8,1,P.wood,g);
  const pennant=flat([[0,0],[2.8,-.5],[1.8,-1.4],[2.8,-2.8],[0,-2.8]],P.faction,g);pennant.position.set(2.6,9.8,0);
  return g;
};
