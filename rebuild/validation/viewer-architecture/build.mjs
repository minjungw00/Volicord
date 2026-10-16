import {build} from 'esbuild';
await build({entryPoints:['vue.js'],bundle:true,minify:true,format:'iife',platform:'browser',target:'es2020',outfile:'app.js',alias:{vue:'vue/dist/vue.runtime.esm-bundler.js'},
  define:{'process.env.NODE_ENV':'"production"',__VUE_OPTIONS_API__:'false',__VUE_PROD_DEVTOOLS__:'false',__VUE_PROD_HYDRATION_MISMATCH_DETAILS__:'false'},
  legalComments:'eof',metafile:true}).then(async r=>{const fs=await import('node:fs');fs.writeFileSync('metafile.json',JSON.stringify(r.metafile,null,2));});
