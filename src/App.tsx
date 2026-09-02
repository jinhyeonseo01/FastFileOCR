import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { open, save } from '@tauri-apps/plugin-dialog';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
const htmlSchema = {...defaultSchema, attributes:{...defaultSchema.attributes,td:['rowSpan','colSpan','align'],th:['rowSpan','colSpan','align']}};
import { ArrowDownToLine, ArrowUpRight, Check, CheckCheck, ChevronDown, ClipboardPaste, Copy, FileImage, FilePlus2, Files, FileText, FolderOpen, LayoutList, LoaderCircle, Maximize2, Minus, Play, Plus, RotateCcw, ScanLine, Search, Settings2, Pause, Download, ShieldCheck, Square, Table2, Trash2, X, ZoomIn } from 'lucide-react';
import type { Page, Settings, Snapshot, DownloadProgress } from './types';

const defaults: Settings = { mode: 'document', instructions: '', device: 'auto', maxTokens: 8192, useLayout:false };
const labels: Record<string,string> = {queued:'대기',processing:'스캔 중',done:'완료',error:'실패'};
const blockLabels: Record<string,string> = {heading:'제목',paragraph:'문단',list:'목록',table:'표',code:'코드',quote:'인용',html:'HTML',separator:'구분선'};
const native = isTauri();
const empty: Snapshot = {project:{id:'',name:'새 문서',settings:defaults,pages:[]},directory:'',busy:false,message:'문서를 추가하고 전체 스캔을 시작하세요.',resourcesReady:false,download:{status:'idle',file:'',downloaded:0,total:1817539616,bytesPerSecond:0}};
function IconButton({title, children, ...props}: React.ButtonHTMLAttributes<HTMLButtonElement> & {title:string}) { return <button className="icon-button" title={title} aria-label={title} {...props}>{children}</button>; }

export default function App() {
  const [data,setData] = useState<Snapshot>(empty);
  const [selected,setSelected] = useState('');
  const [selectedIds,setSelectedIds] = useState<string[]>([]);
  const selectionAnchor = useRef('');
  const [settings,setSettings] = useState<Settings>(defaults);
  const [drafts,setDrafts] = useState<Record<string,string>>({});
  const [image,setImage] = useState('');
  const [tab,setTab] = useState<'preview'|'edit'|'structure'|'raw'>('preview');
  const [modal,setModal] = useState(false);
  const [error,setError] = useState('');
  const [toast,setToast] = useState('');
  const [pending,setPending] = useState(false);
  const [drag,setDrag] = useState(false);
  const [downloadHidden,setDownloadHidden] = useState(false);
  const [query,setQuery] = useState('');
  const [zoom,setZoom] = useState(100);
  const [showRegions,setShowRegions] = useState(true);
  const [activeRegion,setActiveRegion] = useState('');
  const [format,setFormat] = useState('md');
  const [scope,setScope] = useState('all');
  const dataRef = useRef(data); dataRef.current = data;
  const draftsRef = useRef(drafts); draftsRef.current = drafts;
  const operationRef = useRef(false);
  const importRef = useRef<(paths:string[])=>void>(()=>{});
  const pages = data.project.pages;
  const page = pages.find(p=>p.id===selected) ?? pages[0];
  const text = page ? drafts[page.id] ?? page.markdown : '';
  const working = data.busy || pending;
  const completed = pages.filter(p=>p.status==='done').length;
  const failed = pages.filter(p=>p.status==='error').length;
  const scanIds = pages.filter(p=>selectedIds.includes(p.id)).map(p=>p.id);

  const refresh = useCallback(async () => {
    if (!native) return;
    const next = await invoke<Snapshot>('snapshot');
    const previous = dataRef.current;
    setSelectedIds(old=>previous.project.id!==next.project.id || previous.project.pages.length===0 ? next.project.pages.slice(0,1).map(p=>p.id) : old.filter(id=>next.project.pages.some(p=>p.id===id)));
    setData(next);
    setSelected(old=>next.project.pages.some(p=>p.id===old) ? old : next.project.pages[0]?.id ?? '');
  },[]);
  const flush = useCallback(async () => {
    const entries = Object.entries(draftsRef.current);
    for (const [pageId, markdown] of entries) await invoke('edit_page',{pageId,markdown});
    setDrafts(current => {
      const next = {...current};
      for (const [id,text] of entries) if(next[id]===text) delete next[id];
      return next;
    });
  },[]);
  const action = useCallback(async (task:()=>Promise<void>, shouldFlush=true) => {
    if (!native) { setError('실제 파일 OCR은 데스크톱 앱에서 사용할 수 있습니다. npm run desktop으로 실행하세요.'); return; }
    if(operationRef.current) return;
    operationRef.current=true;
    setPending(true); setError('');
    try { if(shouldFlush) await flush(); await task(); await refresh(); }
    catch(e) { setError(String(e)); }
    finally { operationRef.current=false; setPending(false); }
  },[flush,refresh]);
  const addPaths = useCallback((paths:string[]) => { void action(async()=>{
    const errors = await invoke<string[]>('import_paths',{paths});
    if(errors.length) setError(errors.join('\n'));
  }); },[action]);
  importRef.current = addPaths;
  const addFiles = () => void action(async()=>{
    const paths = await open({multiple:true,filters:[{name:'문서 및 이미지',extensions:['pdf','png','jpg','jpeg','webp','bmp']}]});
    if(paths) {
      const errors = await invoke<string[]>('import_paths',{paths:Array.isArray(paths)?paths:[paths]});
      if(errors.length) setError(errors.join('\n'));
    }
  });
  const paste = () => void action(async()=>{await invoke('paste_image');});
  const scan = (ids?:string[]) => void action(async()=>{
    await invoke('update_settings',{settings});
    await invoke('start_scan',{pageIds:ids??null});
  });
  const exportFile = () => void action(async()=>{
    const path = await save({defaultPath:data.project.name+'.'+format,filters:[{name:format.toUpperCase(),extensions:[format]}]});
    if(path) {await invoke('export_document',{path,format});setToast('문서를 내보냈습니다.');}
  });
  const selectPage = (p:Page, event:React.MouseEvent | React.KeyboardEvent) => {
    const additive = event.ctrlKey || event.metaKey;
    setSelectedIds(current=>{
      if(event.shiftKey) {
        const anchorIndex = filtered.findIndex(item=>item.id===selectionAnchor.current);
        const end = filtered.findIndex(item=>item.id===p.id);
        const start = anchorIndex<0 ? end : anchorIndex;
        const range = filtered.slice(Math.min(start,end),Math.max(start,end)+1).map(item=>item.id);
        return additive ? [...new Set([...current,...range])] : range;
      }
      return additive ? current.includes(p.id) ? current.filter(id=>id!==p.id) : [...current,p.id] : [p.id];
    });
    if(!event.shiftKey || !selectionAnchor.current) selectionAnchor.current=p.id;
    setSelected(p.id);setZoom(100);
    if(Object.keys(draftsRef.current).length) void action(async()=>{});
  };
  const selectAll = () => {setSelectedIds(filtered.map(p=>p.id));selectionAnchor.current=filtered[0]?.id??'';};
  const selectionKeys = (event:React.KeyboardEvent) => {
    const target=event.target as HTMLElement;
    if(target.closest('input,textarea,[contenteditable="true"]')) return;
    if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='a') {event.preventDefault();selectAll();}
    if(event.key==='Escape') {event.preventDefault();setSelectedIds([]);}
    if((event.key==='ArrowDown'||event.key==='ArrowUp')&&filtered.length) {
      event.preventDefault();
      const index=filtered.findIndex(p=>p.id===page?.id);
      const next=filtered[Math.max(0,Math.min(filtered.length-1,index+(event.key==='ArrowDown'?1:-1)))];
      selectPage(next,event);
      document.getElementById('page-'+next.id)?.focus();
    }
  };
  const newProject = () => void action(async()=>{
    const parent = await open({directory:true,title:'새 작업을 저장할 폴더'});
    if(typeof parent==='string') await invoke('create_project',{parent,name:'새 문서'});
  });
  const openProject = () => void action(async()=>{
    const directory = await open({directory:true,title:'project.json이 있는 Glyph 작업 폴더'});
    if(typeof directory==='string') await invoke('open_project',{directory});
  });

  useEffect(()=>{
    if(!native) return;
    let active=true;
    void refresh().catch(e=>setError(String(e)));
    const undownload = listen<DownloadProgress>('model-download',({payload})=>{if(active){setData(current=>({...current,download:payload}));if(['downloading','checking'].includes(payload.status))setDownloadHidden(false);}});
    const unlisten = listen('workspace-changed',()=>{if(active) void refresh().catch(e=>setError(String(e)));});
    const undrag = getCurrentWebviewWindow().onDragDropEvent(({payload})=>{
      if(!active) return;
      if(payload.type==='enter') setDrag(true);
      if(payload.type==='leave') setDrag(false);
      if(payload.type==='drop') {setDrag(false);if(!dataRef.current.busy) importRef.current(payload.paths);}
    });
    return ()=>{active=false;void unlisten.then(f=>f());void undrag.then(f=>f());void undownload.then(f=>f());};
  },[refresh]);
  useEffect(()=>{setSettings(data.project.settings);setDrafts({});},[data.project.id]);
  useEffect(()=>{
    setImage('');setActiveRegion('');
    if(!page||!native) return;
    let active=true;
    invoke<string>('preview',{pageId:page.id}).then(url=>{if(active)setImage(url);}).catch(e=>{if(active)setError(String(e));});
    return ()=>{active=false;};
  },[page?.id]);
  useEffect(()=>{if(!toast)return;const t=setTimeout(()=>setToast(''),3000);return()=>clearTimeout(t);},[toast]);
  useEffect(()=>{
    const handler=(event:KeyboardEvent)=>{
      const target=event.target as HTMLElement;
      if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='v'&&!['INPUT','TEXTAREA'].includes(target.tagName)&&!working) {
        event.preventDefault();paste();
      }
    };
    const pasted=(event:ClipboardEvent)=>{
      if(working)return;
      const hasImage=Array.from(event.clipboardData?.items??[]).some(item=>item.type.startsWith('image/'));
      if(hasImage){event.preventDefault();paste();}
    };
    window.addEventListener('keydown',handler);window.addEventListener('paste',pasted);
    return()=>{window.removeEventListener('keydown',handler);window.removeEventListener('paste',pasted);};
  });
  const download = data.download ?? empty.download;
  const downloadActive = ['downloading','checking','pausing','paused'].includes(download.status);
  const filtered = pages.filter(p=>p.name.toLowerCase().includes(query.toLowerCase())&&(scope==='all'||p.status===scope));

  return <div className="app-shell">
    <aside className="sidebar" tabIndex={-1} onKeyDown={selectionKeys} onMouseDown={e=>{if(!(e.target as HTMLElement).closest('button,input,select,textarea'))e.currentTarget.focus();}}>
      <div className="brand"><div className="brand-icon"><ScanLine size={24}/></div><span>glyph<span className="brand-dot">.</span><small className="brand-ocr">OCR</small></span><span className="version">2.0</span></div>
      <div className="workspace-label">WORKSPACE <span>로컬 작업</span></div>
      <div className="project-card"><div className="project-symbol"><Files size={20}/></div><div><strong>{data.project.name}</strong><span>{pages.length}페이지 · 자동 저장</span></div><IconButton title="작업 폴더 열기" disabled={!native} onClick={()=>void action(async()=>{await invoke('open_folder');},false)}><ArrowUpRight size={16}/></IconButton></div>
      <div className="project-actions"><button disabled={working} onClick={newProject}><Plus size={14}/>새 작업</button><button disabled={working} onClick={openProject}><FolderOpen size={14}/>불러오기</button></div>
      <div className="section-label">문서 목록 <span>{pages.length.toString().padStart(2,'0')}</span><IconButton title="문서 추가" disabled={working} onClick={addFiles}><Plus size={16}/></IconButton></div>
      <label className="search"><Search size={15}/><input aria-label="문서 검색" placeholder="문서 검색" value={query} onChange={e=>setQuery(e.target.value)}/></label>
      {pages.length>0&&<div className="filter-row"><button className={scope==='all'?'active':''} onClick={()=>setScope('all')}>전체</button><button className={scope==='done'?'active':''} onClick={()=>setScope('done')}>완료 {completed}</button>{failed>0&&<button className={scope==='error'?'active':''} onClick={()=>setScope('error')}>실패 {failed}</button>}</div>}
      {pages.length>0&&<div className="selection-toolbar"><button onClick={selectAll} disabled={!filtered.length}>전체 선택</button><span aria-live="polite">{scanIds.length}개 선택</span><button onClick={()=>setSelectedIds([])} disabled={!scanIds.length}>해제</button></div>}
      <div className="page-list" role="listbox" aria-label="스캔할 페이지" aria-multiselectable="true" tabIndex={0}>{filtered.map((p)=><button id={'page-'+p.id} role="option" aria-selected={selectedIds.includes(p.id)} key={p.id} className={'page-row '+(selectedIds.includes(p.id)?'selected ':'')+(p.id===page?.id?'current':'')} onClick={event=>selectPage(p,event)}>
        <span className="selection-check" aria-hidden="true">{selectedIds.includes(p.id)&&<Check size={11}/>}</span>
        <div className={'page-icon '+p.status}>{p.status==='processing'?<LoaderCircle size={20} className="spin"/>:<FileText size={20}/>}</div><div className="page-meta"><strong title={p.name}>{p.name}</strong><span>{p.width} × {p.height}<i/> {labels[p.status]}</span></div>{p.status==='done'&&<Check size={14} className="green"/>}
      </button>)}
      {pages.length===0&&<div className="list-placeholder"><div/><div/><div/><p>추가한 문서가 여기에 표시됩니다.</p></div>}
      {pages.length>0&&filtered.length===0&&<p className="muted empty-search">검색 결과가 없습니다.</p>}
      </div>
      {pages.length>0&&<p className="selection-hint">Shift 범위 · Ctrl 개별 · Ctrl+A 전체</p>}
      <button className="sidebar-add" onClick={addFiles} disabled={working}><Plus size={17}/>문서 추가</button>
      <div className="privacy-note"><ShieldCheck size={18}/><div><strong>내 기기에서만, 안전하게</strong><p>문서와 인식 결과는 외부로 전송되지 않습니다.</p></div></div>
      <div className="sidebar-footer"><span className={'status-dot '+(data.resourcesReady?'ready':'')}/>{data.resourcesReady?(download.status==='ready'?'오프라인 엔진 준비됨':'첫 스캔 시 모델 준비'):'데스크톱 문서 OCR'}<button onClick={()=>setModal(true)} aria-label="설정"><Settings2 size={16}/></button></div>
    </aside>

    <main className="main">
      <header className="topbar"><div className="breadcrumb">작업 공간 <span>/</span><strong>문서 스캔</strong></div><div className="top-actions"><span className="local-pill"><span/>100% LOCAL</span><IconButton title="인식 설정" onClick={()=>setModal(true)}><Settings2 size={19}/></IconButton><div className="export-group"><select aria-label="내보내기 형식" value={format} onChange={e=>setFormat(e.target.value)}><option value="md">Markdown</option><option value="txt">TXT</option><option value="json">JSON</option><option value="html">HTML</option></select><button onClick={exportFile} disabled={working||!pages.some(p=>p.markdown)}><ArrowDownToLine size={16}/>내보내기</button></div></div></header>
      <section className="heading"><div><div className="eyebrow">DOCUMENT INTELLIGENCE, ON YOUR DEVICE</div><h1>문서에서, 바로 텍스트로<span>.</span></h1><p>스캔본부터 사진과 화면 캡처까지. 읽고, 정리하고, 필요한 형식으로 저장하세요.</p></div><div className="heading-stamp"><ScanLine size={25}/><span>FULL PAGE<br/><strong>한 페이지를 한 번에</strong></span></div></section>
      {!native&&<div className="browser-notice">화면 미리보기입니다. 파일 인식은 <code>npm run desktop</code>으로 실행한 앱에서 사용할 수 있습니다.</div>}
      {error&&<div className="error-banner" role="alert"><span>{error}</span><IconButton title="알림 닫기" onClick={()=>setError('')}><X size={16}/></IconButton></div>}
      <section className="command-bar"><div className="command-left"><span className="tiny-label">인식 모드</span><select aria-label="인식 모드" value={settings.mode} disabled={working} onChange={e=>setSettings({...settings,mode:e.target.value as Settings['mode']})}><option value="document">문서 구조 유지</option><option value="text">텍스트 그대로</option><option value="table">표 인식</option><option value="formula">수식 인식</option><option value="comic">만화 · 말풍선</option></select><button className={'instruction-button '+(settings.instructions?'has-instructions':'')} onClick={()=>setModal(true)}><Settings2 size={15}/>{settings.instructions?'사용자 지침 적용':'사용자 지침'}</button></div><div className="command-right"><span className="queue-count">{completed} / {pages.length} 페이지</span>{data.busy?<button className="stop-button" onClick={()=>void action(async()=>{await invoke('cancel_scan');},false)}><Square size={14}/>중단</button>:<><button className="secondary-button scan-all" disabled={working||!pages.length} onClick={()=>scan(pages.map(p=>p.id))}>전체 스캔</button><button className="primary-button" disabled={working||!scanIds.length} onClick={()=>scan(scanIds)}><Play size={16} fill="currentColor"/>{pending?'처리 중…':'선택 스캔'}{scanIds.length>0&&<span>{scanIds.length}</span>}</button></>}</div></section>
      <div className="layout-options"><label><input type="checkbox" checked={settings.useLayout??false} disabled={working} onChange={e=>setSettings({...settings,useLayout:e.target.checked})}/>영역 탐지 후 세부 OCR</label><span>{settings.useLayout?'영역별 텍스트 · 위치 · 읽기 순서':'전체 페이지를 한 번에 인식'}</span></div>
      {settings.mode==='comic'&&<p className="mode-hint">말풍선·내레이션·효과음을 읽기 순서대로 추출합니다. 오른쪽→왼쪽 만화 등 읽는 방향은 사용자 지침에 지정하세요.</p>}
      {pages.length===0?<section className="empty-workspace">
        <div className="drop-zone"><div className="document-art"><div className="art-page back"/><div className="art-page front"><div className="art-heading"/><div className="art-line"/><div className="art-line short"/><div className="art-table"/><div className="scan-beam"/></div><div className="art-badge"><ScanLine size={21}/></div></div><div className="eyebrow">YOUR DOCUMENTS, YOUR DEVICE</div><h2>읽고 싶은 문서를 놓아주세요</h2><p>PDF, 문서 사진, 스캔 이미지, PC 캡처를 끌어다 놓으세요.<br/>여러 파일도 한 번에 추가할 수 있습니다.</p><div className="empty-buttons"><button className="primary-button" onClick={addFiles} disabled={working}><FilePlus2 size={17}/>파일 선택</button><button className="secondary-button" onClick={paste} disabled={working}><ClipboardPaste size={16}/>캡처 붙여넣기 <kbd>Ctrl V</kbd></button></div><div className="file-types"><span>PDF</span><span>PNG</span><span>JPG</span><span>WEBP</span><span>BMP</span></div></div>
        <div className="feature-grid"><div><div className="feature-icon"><ScanLine size={20}/></div><h3>페이지 전체를 스캔</h3><p>페이지의 흐름을 그대로 읽어<br/>문서의 내용을 한 번에 추출합니다.</p></div><div><div className="feature-icon"><LayoutList size={20}/></div><h3>문서 구조까지 함께</h3><p>제목, 문단, 목록과 표를 유지하고<br/>결과를 직접 검토하고 편집하세요.</p></div><div><div className="feature-icon"><ArrowDownToLine size={20}/></div><h3>필요한 형태로 저장</h3><p>TXT, Markdown, HTML, JSON으로<br/>다음 작업에 바로 이어가세요.</p></div></div>
      </section>:<section className="editor-workspace">
        <div className="source-panel"><div className="panel-header"><span><FileImage size={16}/>원본 페이지{Boolean(page?.regions?.length)&&<button className="region-toggle" aria-pressed={showRegions} onClick={()=>setShowRegions(!showRegions)}>영역 {showRegions?'숨기기':'보기'}</button>}</span><div><IconButton title="축소" onClick={()=>setZoom(Math.max(25,zoom-25))}><Minus size={15}/></IconButton><span className="zoom-label">{zoom}%</span><IconButton title="확대" onClick={()=>setZoom(Math.min(300,zoom+25))}><ZoomIn size={15}/></IconButton><IconButton title="화면에 맞추기" onClick={()=>setZoom(100)}><Maximize2 size={15}/></IconButton></div></div><div className="source-canvas">{image?<div className="page-image" style={{width:zoom+'%',maxWidth:zoom===100?'100%':'none'}}><img src={image} alt={page?.name}/>{showRegions&&!!page?.regions?.length&&<svg className="region-overlay" viewBox={'0 0 '+page.width+' '+page.height} aria-label="탐지된 문서 영역">{page.regions.map(r=><g key={r.id} className={activeRegion===r.id?'active':''} onClick={()=>{setActiveRegion(r.id);setTab('structure');}}><title>{r.order+'. '+r.label+' · '+r.markdown.slice(0,100)}</title><rect x={r.bbox[0]} y={r.bbox[1]} width={r.bbox[2]-r.bbox[0]} height={r.bbox[3]-r.bbox[1]}/><text x={r.bbox[0]+4} y={r.bbox[1]+22} fontSize={Math.max(18,page.width/65)}>{r.order}</text></g>)}</svg>}</div>:<LoaderCircle className="spin" size={28}/>}</div><div className="panel-footer"><span title={page?.name}>{page?.name}</span><span>{page?.width} × {page?.height}</span></div></div>
        <div className="result-panel"><div className="panel-header"><div className="tabs">{([['preview','문서'],['edit','편집'],['structure','구조'],['raw','원문']] as const).map(([id,name])=><button key={id} className={tab===id?'active':''} onClick={()=>setTab(id)}>{name}{id==='edit'&&page&&drafts[page.id]!==undefined&&<i/>}</button>)}</div><div><IconButton title="결과 복사" disabled={!text} onClick={()=>void action(async()=>{await invoke('copy_text',{text:tab==='raw'?page?.rawText:text});setToast('클립보드에 복사했습니다.');})}><Copy size={15}/></IconButton><IconButton title="이 페이지 다시 스캔" disabled={working} onClick={()=>page&&scan([page.id])}><RotateCcw size={15}/></IconButton><IconButton title="페이지 제거" disabled={working} onClick={()=>void action(async()=>{await invoke('remove_page',{pageId:page?.id});})}><Trash2 size={15}/></IconButton></div></div>
        {page?.error&&<div className="page-alert error">{page.error}</div>}{page?.warning&&<div className="page-alert">{page.warning}</div>}
        <div className="result-content">
          {tab==='edit'?<textarea aria-label="인식 결과 편집" className="result-editor" spellCheck={false} value={text} disabled={working} onChange={e=>page&&setDrafts({...drafts,[page.id]:e.target.value})} placeholder="스캔 결과가 여기에 표시됩니다. 직접 입력하거나 수정할 수 있습니다."/>:
          tab==='raw'?<pre className="raw-output">{page?.rawText||'모델이 반환한 원문이 여기에 보존됩니다.'}</pre>:
          tab==='structure'?<div className="structure-view"><p className="structure-note">{page?.regions?.length?'원본의 영역을 누르면 해당 위치와 인식 결과를 확인할 수 있습니다. 좌표는 표시된 페이지 이미지의 픽셀 기준입니다.':'인식 결과에서 추출한 문서 구조입니다. 영역 탐지 후 세부 OCR을 켜면 영역 좌표도 함께 저장합니다.'}</p>{page?.regions?.map(r=><button className={'region-result '+(activeRegion===r.id?'active':'')} key={r.id} onClick={()=>setActiveRegion(r.id)}><strong>{r.order}. {r.label} <small>{(r.confidence*100).toFixed(0)}%</small></strong><span>{r.bbox.map(n=>Math.round(n)).join(', ')} px · {r.status==='skipped'?'비텍스트 영역':r.ocrMode}</span><p>{r.markdown||'인식된 텍스트 없음'}</p>{r.warning&&<em className="region-warning">{r.warning}</em>}</button>)}{page?.blocks.map((block,index)=><div className="structure-block" key={index}><span>{String(index+1).padStart(2,'0')}</span><div><strong>{blockLabels[block.kind]??block.kind}{block.level?' H'+block.level:''}{block.rows?' · '+block.rows.length+'행':''}</strong><p>{block.text}</p></div></div>)}{!page?.blocks.length&&<p className="muted">스캔 후 문서 구조를 확인할 수 있습니다.</p>}</div>:
          text&&page?.recognizedWith?.mode==='text'?<pre className="raw-output">{text}</pre>:text?<article className="markdown"><Markdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeRaw,[rehypeSanitize,htmlSchema]]} components={{img:()=>null,a:({children})=><span className="document-link">{children}</span>}}>{text}</Markdown></article>:
          <div className="result-empty"><div className="result-empty-icon">{page?.status==='processing'?<LoaderCircle size={28} className="spin"/>:<ScanLine size={28}/>}</div><h3>{page?.status==='processing'?'페이지를 읽고 있습니다':'텍스트를 만날 준비가 됐어요'}</h3><p>{page?.status==='processing'?'완료하면 결과가 자동으로 저장됩니다.':'페이지를 선택하고 스캔하세요. 완료 후에도 모드와 지침을 바꿔 다시 인식할 수 있습니다.'}</p></div>}
        </div><div className="panel-footer"><span>{text.length.toLocaleString()}자{page?.elapsedMs?' · '+(page.elapsedMs/1000).toFixed(1)+'초':''}</span>{page&&drafts[page.id]!==undefined?<button className="text-button" disabled={working} onClick={()=>void action(async()=>{setToast('수정 내용을 저장했습니다.');})}><Check size={14}/>변경 저장</button>:<span className="saved-label"><CheckCheck size={13}/>자동 저장</span>}</div></div>
      </section>}
      <footer className="statusbar"><div>{working?<LoaderCircle size={13} className="spin"/>:<span className="status-dot ready"/>}<span>{pending?'작업 처리 중…':data.message}</span></div><span>PaddleOCR-VL 1.6 <span className="footer-divider">/</span> llama.cpp</span></footer>
    </main>
    {drag&&<div className="drag-overlay"><ScanLine size={44}/><h2>문서를 놓아 추가하세요</h2><p>PDF · PNG · JPG · WEBP · BMP</p></div>}
    {!downloadHidden&&!['idle','ready'].includes(download.status)&&<section className="download-popup" role="region" aria-label="모델 다운로드"><div className="download-heading"><Download size={18}/><strong>{({checking:'모델 파일 확인 중',downloading:'모델 다운로드 중',pausing:'다운로드 일시정지 중',paused:'다운로드 일시정지',cancelled:'다운로드 중단됨',interrupted:'다운로드 이어받기',error:'다운로드 오류'} as Record<string,string>)[download.status]}</strong>{!downloadActive&&<IconButton title="다운로드 알림 닫기" onClick={()=>setDownloadHidden(true)}><X size={15}/></IconButton>}</div><p className="download-filename">{download.file||'PaddleOCR-VL 1.6'}</p><progress aria-label="모델 다운로드 진행률" value={download.downloaded} max={download.total}/><div className="download-numbers"><span>{(download.downloaded/1e6).toFixed(1)} / {(download.total/1e6).toFixed(1)} MB</span><strong>{Math.min(100,download.downloaded/download.total*100).toFixed(1)}%</strong></div>{download.bytesPerSecond>0&&<p className="download-speed">{(download.bytesPerSecond/1e6).toFixed(1)} MB/s</p>}{download.error&&<p className="download-error">{download.error}</p>}<div className="download-actions">{downloadActive?<>{download.status==='paused'?<button onClick={()=>void invoke('resume_download').catch(e=>setError(String(e)))}><Play size={13}/>계속 받기</button>:<button disabled={download.status==='pausing'} onClick={()=>void invoke('pause_download').catch(e=>setError(String(e)))}><Pause size={13}/>일시정지</button>}<button onClick={()=>void action(async()=>{await invoke('cancel_scan');},false)}><Square size={13}/>중단</button></>:<button disabled={working||!pages.length} onClick={()=>scan(scanIds.length?scanIds:pages.map(p=>p.id))}><Play size={13}/>이어받고 스캔</button>}</div><p className="download-footnote">받은 파일은 보관됩니다. 다음 실행에서도 이어받을 수 있습니다.</p></section>}
    {toast&&<div role="status" className="toast"><Check size={16}/>{toast}</div>}
    {modal&&<div className="modal-backdrop" onMouseDown={e=>{if(e.target===e.currentTarget)setModal(false);}}><section className="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title"><div className="modal-header"><div><div className="eyebrow">SCAN PREFERENCES</div><h2 id="settings-title">문서를 읽는 방식</h2></div><IconButton title="설정 닫기" onClick={()=>setModal(false)}><X size={20}/></IconButton></div><label className="field">사용자 인식 지침<textarea value={settings.instructions} maxLength={4000} disabled={working} onChange={e=>setSettings({...settings,instructions:e.target.value})} placeholder="예: 한국어와 영어를 원문 그대로 유지하고, 표의 열 순서를 보존해 주세요."/></label><p className="field-hint">지침은 선택한 방식의 OCR 요청에 함께 전달됩니다. 복잡한 지침과 레이아웃의 반영 정도는 문서에 따라 다를 수 있습니다.</p><div className="settings-grid"><label className="field">실행 장치<select disabled={working} value={settings.device} onChange={e=>setSettings({...settings,device:e.target.value as Settings['device']})}><option value="auto">자동 · GPU 우선</option><option value="vulkan">GPU · Vulkan</option><option value="cpu">CPU · 호환 모드</option></select></label><label className="field">최대 출력 길이<select disabled={working} value={settings.maxTokens} onChange={e=>setSettings({...settings,maxTokens:Number(e.target.value)})}><option value={4096}>4,096 토큰</option><option value={8192}>8,192 토큰</option><option value={16384}>16,384 토큰</option></select></label></div><div className="settings-info"><ShieldCheck size={18}/><p>실행 엔진은 앱에 포함됩니다. 모델은 첫 스캔 때 Hugging Face에서 약 1.82GB를 받으며, 영역 탐지를 켜면 약 133MB를 추가로 받습니다. 이후 OCR은 기기 안에서 처리하며 문서를 업로드하지 않습니다.</p></div><div className="modal-footer"><button className="secondary-button" onClick={()=>setModal(false)}>닫기</button><button className="primary-button" disabled={working} onClick={()=>void action(async()=>{await invoke('update_settings',{settings});setModal(false);setToast('인식 설정을 저장했습니다.');})}>설정 저장<Check size={16}/></button></div></section></div>}
  </div>;
}
