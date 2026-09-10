// Approved original marker at local origin, in world pixels.
window.ReferenceModel=function(kit){
  const {T,P,DECAL,box,tapered,cutBox,flat}=kit,g=new T.Group();
  cutBox(0,0,0,5.6,1,4.2,1,P.dark,g);
  tapered(0,1,0,4.2,3.5,2.8,2.8,3.2,P.faction,g);
  cutBox(0,4.2,0,2.8,2.8,2.8,.6,P.skin,g);
  tapered(0,6.4,0,5.6,4.2,5.6,4.2,.4,P.roofDark,g);
  tapered(0,6.8,0,3.5,3,2,1.8,1.6,P.roof,g);
  return g;
};
