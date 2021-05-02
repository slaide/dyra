#version
1
#settings
vertex_coordinate_count=3
vertex_texture_coordinates=true
vertex_texture_coordinate_count=2
vertex_count=4
face_count=2
vertex_data_interleaved_input=true
index_start=1
#defaults
v.z=0.0
v.w=1.0
vt.w=1.0
#vertexdata
v -0.7 -0.7 0.0
vt 0.0 0.0
v -0.7 0.7 0.0
vt 0.0 1.0
v 0.7 -0.7 0.0
vt 1.0 0.0
v 0.7 0.7 0.0
vt 1.0 1.0
#facedata
f 1 2 3
f 2 4 3