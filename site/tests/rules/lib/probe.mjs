// The in-page probe: one page.evaluate() that reads every computed style the rendered checks
// need, so each page is measured once per viewport and the rules are judged in Node
// (lib/judge.mjs). The function below runs in the browser and must stay self-contained.

/* eslint-disable no-undef */
export function probePage() {
  const cs = (el) => getComputedStyle(el);
  const px = (v) => (v === 'normal' || v === '' ? null : parseFloat(v));
  const vw = innerWidth;
  const vh = innerHeight;
  let qid = 0;

  const describe = (el) => {
    const parts = [];
    for (let e = el, i = 0; e && e !== document.body && i < 4; e = e.parentElement, i++) {
      let s = e.tagName.toLowerCase();
      if (e.id) s += `#${e.id}`;
      else if (e.classList.length) s += `.${[...e.classList].filter((c) => !c.startsWith('astro-')).slice(0, 2).join('.')}`;
      parts.unshift(s.replace(/\.$/, ''));
    }
    return parts.join(' > ');
  };
  const ownText = (el) =>
    [...el.childNodes]
      .filter((n) => n.nodeType === 3)
      .map((n) => n.textContent)
      .join(' ')
      .replace(/\s+/g, ' ')
      .trim();
  const isSrOnly = (el) => {
    for (let e = el; e && e !== document.body; e = e.parentElement) {
      const s = cs(e);
      const r = e.getBoundingClientRect();
      if (s.position === 'absolute' && r.width <= 1.5 && r.height <= 1.5) return true;
      if (s.clip && s.clip !== 'auto' && /rect\(0(px)?,? 0(px)?,? 0(px)?,? 0(px)?\)/.test(s.clip)) return true;
      if (/inset\(50%\)/.test(s.clipPath)) return true;
    }
    return false;
  };
  const isVisible = (el) => {
    const s = cs(el);
    if (s.display === 'none' || s.visibility === 'hidden') return false;
    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) return false;
    // Parked off the page until focused, like a skip link.
    if (r.bottom + scrollY <= 0 || r.right + scrollX <= 0) return false;
    for (let e = el; e; e = e.parentElement) if (cs(e).display === 'none') return false;
    return true;
  };
  const opacityProduct = (el) => {
    let o = 1;
    for (let e = el; e; e = e.parentElement) o *= parseFloat(cs(e).opacity);
    return o;
  };
  const bgLayers = (el) => {
    const layers = [];
    for (let e = el; e; e = e.parentElement) {
      const s = cs(e);
      layers.push({ color: s.backgroundColor, image: s.backgroundImage !== 'none', backdrop: s.backdropFilter !== 'none' });
    }
    return layers;
  };
  const rect = (el) => {
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top + scrollY, w: r.width, h: r.height, top: r.top, bottom: r.bottom + scrollY };
  };
  // A filled, rounded box inside <main>: a card even when nobody marked it with data-card.
  const cardLike = (el) => {
    for (let e = el.parentElement; e && e.tagName !== 'MAIN' && e !== document.body; e = e.parentElement) {
      const s = cs(e);
      if (parseFloat(s.borderTopLeftRadius) >= 12 && !/rgba\(0, 0, 0, 0\)|transparent/.test(s.backgroundColor)) return true;
    }
    return false;
  };
  const ctx = (el) => ({
    inMain: !!el.closest('main'),
    inHeader: !!el.closest('header') && !el.closest('main, article'),
    inFooter: !!el.closest('footer') && !el.closest('main'),
    inDocs: !!el.closest('.sl-markdown-content, [data-docs-prose]'),
    inCard: !!el.closest('[data-card]') || (!!el.closest('main') && cardLike(el)),
    inNav: !!el.closest('nav'),
    tier: !!el.closest('[data-tier]'),
  });
  const lineCount = (el) => {
    const range = document.createRange();
    range.selectNodeContents(el);
    const tops = new Set();
    for (const r of range.getClientRects()) if (r.width > 1 && r.height > 1) tops.add(Math.round(r.top));
    // Merge tops closer than half a line (inline boxes of different fonts on one line).
    const lh = px(cs(el).lineHeight) ?? px(cs(el).fontSize) * 1.2;
    const sorted = [...tops].sort((a, b) => a - b);
    let lines = 0;
    let last = -Infinity;
    for (const t of sorted) {
      if (t - last > lh / 2) lines++;
      last = t;
    }
    return lines;
  };
  const numericText = (t) => /^[−+-]?\d[\d,.    ]*(%|×)?$/.test(t);
  const SKIP = new Set(['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE', 'svg', 'SVG', 'path', 'TITLE', 'HEAD', 'META', 'LINK']);
  const all = [...document.body.querySelectorAll('*')].filter((el) => !SKIP.has(el.tagName) && !el.closest('svg'));

  const htmlStyle = cs(document.documentElement);
  const bodyStyle = cs(document.body);
  const out = {
    url: location.href,
    viewport: { w: vw, h: vh },
    scrollHeight: document.documentElement.scrollHeight,
    html: {
      bg: htmlStyle.backgroundColor,
      colorScheme: htmlStyle.colorScheme,
      metaColorScheme: document.querySelector('meta[name="color-scheme"]')?.content ?? null,
      themeColor: document.querySelector('meta[name="theme-color"]')?.content ?? null,
      dataTheme: document.documentElement.dataset.theme ?? null,
    },
    body: { bg: bodyStyle.backgroundColor, color: bodyStyle.color, fontFamily: bodyStyle.fontFamily },
    fontsLoaded: [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family.replace(/["']/g, '')),
    mainTextLength: (document.querySelector('main')?.innerText ?? '').trim().length,
    h1Text: document.querySelector('h1')?.innerText.trim() ?? null,
    navLinks: document.querySelectorAll('header nav a[href], nav[aria-label] a[href]').length,
    isDocs: location.pathname.startsWith('/docs/') || !!document.querySelector('[data-docs-page]'),
  };

  // Header (rule 11).
  const header = document.querySelector('body > header, body > div > header, header');
  if (header && !header.closest('main')) {
    const s = cs(header);
    out.header = { path: describe(header), height: header.getBoundingClientRect().height, position: s.position, bg: s.backgroundColor, backdrop: s.backdropFilter, top: header.getBoundingClientRect().top };
  } else out.header = null;

  // Headings (rules 2, 3, 4, 9, 24).
  out.headings = [...document.querySelectorAll('h1, h2, h3')].filter(isVisible).map((h) => {
    const s = cs(h);
    const grads = [...h.querySelectorAll('*'), h].filter((e) => {
      const st = cs(e);
      return (st.backgroundClip === 'text' || st.webkitBackgroundClip === 'text') && st.backgroundImage !== 'none';
    });
    const last = h.lastElementChild;
    return {
      path: describe(h),
      tag: h.tagName.toLowerCase(),
      text: h.innerText.trim().slice(0, 80),
      fontFamily: s.fontFamily,
      fontWeight: parseFloat(s.fontWeight),
      fontSize: px(s.fontSize),
      lineHeight: px(s.lineHeight),
      letterSpacing: px(s.letterSpacing) ?? 0,
      color: s.color,
      lines: lineCount(h),
      gradients: grads.map((g) => ({ text: g.innerText.trim().slice(0, 60), image: cs(g).backgroundImage, isLast: g === last && !last.nextSibling?.textContent?.trim(), self: g === h })),
      rect: rect(h),
      ...ctx(h),
    };
  });

  // Gradient text anywhere (rule 9).
  out.gradientText = all
    .filter((e) => {
      const st = cs(e);
      return (st.backgroundClip === 'text' || st.webkitBackgroundClip === 'text') && st.backgroundImage !== 'none' && isVisible(e);
    })
    .map((e) => ({ path: describe(e), text: e.innerText.trim().slice(0, 60), image: cs(e).backgroundImage, inHeading: !!e.closest('h1, h2, h3, h4, h5, h6'), heading: e.closest('h1, h2, h3, h4, h5, h6')?.tagName.toLowerCase() ?? null, number: !!e.closest('[data-number]') }));

  // Every element with its own visible text (rules 2, 6, 7, 29).
  out.texts = all
    .filter((e) => ownText(e) && isVisible(e))
    .map((e) => {
      const s = cs(e);
      return {
        path: describe(e),
        tag: e.tagName.toLowerCase(),
        text: ownText(e).slice(0, 60),
        fontFamily: s.fontFamily,
        fontSize: px(s.fontSize),
        fontWeight: parseFloat(s.fontWeight),
        color: s.color,
        textTransform: s.textTransform,
        fontVariantCaps: s.fontVariantCaps,
        opacity: opacityProduct(e),
        bg: bgLayers(e),
        srOnly: isSrOnly(e),
        gradient: (s.backgroundClip === 'text' || s.webkitBackgroundClip === 'text') && s.backgroundImage !== 'none',
        code: !!e.closest('code, pre, kbd, samp'),
        ariaHidden: !!e.closest('[aria-hidden="true"]'),
        inDisplayHeading: !!e.closest('h1, h2'),
        displayNumber: (!!e.closest('[data-number]') || numericText(ownText(e))) && px(s.fontSize) >= 20,
        ...ctx(e),
      };
    });

  // Paragraphs (rules 5, 25).
  out.paragraphs = [...document.querySelectorAll('main p')].filter(isVisible).map((p) => {
    const s = cs(p);
    return { path: describe(p), text: p.innerText.trim().slice(0, 200), words: p.innerText.trim().split(/\s+/).filter(Boolean).length, fontFamily: s.fontFamily, fontSize: px(s.fontSize), lineHeight: px(s.lineHeight), color: s.color, ...ctx(p) };
  });

  // Buttons: links and buttons with a filled background (rules 8, 13).
  const transparent = (c) => /rgba\(0, 0, 0, 0\)|transparent/.test(c) || /\/ 0\)$/.test(c);
  out.buttons = [...document.querySelectorAll('a[href], button, [role="button"], input[type="submit"], input[type="button"]')]
    .filter((b) => isVisible(b) && !isSrOnly(b) && !transparent(cs(b).backgroundColor))
    .map((b) => {
      const s = cs(b);
      const id = `q${++qid}`;
      b.setAttribute('data-quality-id', id);
      return {
        id,
        path: describe(b),
        text: b.innerText.trim().slice(0, 50),
        height: b.getBoundingClientRect().height,
        width: b.getBoundingClientRect().width,
        radius: px(s.borderTopLeftRadius),
        fontSize: px(s.fontSize),
        fontWeight: parseFloat(s.fontWeight),
        bg: s.backgroundColor,
        color: s.color,
        gap: px(s.columnGap),
        icons: [...b.querySelectorAll('svg')].map((i) => ({ w: i.getBoundingClientRect().width, h: i.getBoundingClientRect().height })),
        transitionDuration: s.transitionDuration,
        transitionProperty: s.transitionProperty,
        ...ctx(b),
      };
    });

  // Links (rules 21, 23).
  out.links = [...document.querySelectorAll('a[href]')].filter(isVisible).map((a) => {
    const s = cs(a);
    const url = new URL(a.getAttribute('href'), location.href);
    const hidden = [...a.querySelectorAll('*')].filter((e) => isSrOnly(e));
    return {
      path: describe(a),
      href: url.href,
      host: url.host,
      protocol: url.protocol,
      internal: url.host === location.host || url.host === 'jarrow.pages.dev',
      target: a.getAttribute('target'),
      rel: a.getAttribute('rel'),
      text: [...a.childNodes].map((n) => (n.nodeType === 3 ? n.textContent : n.nodeType === 1 && !isSrOnly(n) && n.tagName.toLowerCase() !== 'svg' ? n.innerText ?? '' : '')).join(' ').replace(/\s+/g, ' ').trim().slice(0, 60),
      srNewTab: hidden.some((e) => /\(opens in a new tab\)/i.test(e.textContent)),
      buttonLike: !transparent(s.backgroundColor),
      brandMark: !!a.querySelector('svg[data-brand-source]'),
      inBody: !!a.closest('main p, main li, main td, main dd, footer p'),
      arrow: !![...a.querySelectorAll('svg')].find((i) => !i.hasAttribute('data-brand-source')) || /[↗→]\s*$/.test(a.textContent.trim()),
      transitionDuration: s.transitionDuration,
      transitionProperty: s.transitionProperty,
      ...ctx(a),
    };
  });

  // Numbers (rule 14).
  out.numbers = all
    .filter((e) => isVisible(e) && (e.hasAttribute('data-number') || (numericText(ownText(e)) && px(cs(e).fontSize) >= 20)))
    .map((e) => ({ path: describe(e), text: (ownText(e) || e.innerText).slice(0, 30), marked: e.hasAttribute('data-number'), fontVariantNumeric: cs(e).fontVariantNumeric, fontSize: px(cs(e).fontSize) }));
  out.units = [...document.querySelectorAll('[data-unit]')].filter(isVisible).map((e) => ({ path: describe(e), text: e.innerText.trim().slice(0, 20), fontSize: px(cs(e).fontSize), color: cs(e).color }));

  // Cards (rule 12): marked with data-card, plus anything in <main> that looks like one.
  out.cards = [...document.querySelectorAll('main *')]
    .filter((e) => {
      if (!isVisible(e) || ['IMG', 'CANVAS', 'PRE', 'CODE', 'INPUT', 'TEXTAREA', 'SELECT', 'BUTTON', 'svg', 'VIDEO'].includes(e.tagName)) return false;
      if (e.hasAttribute('data-card')) return true;
      const s = cs(e);
      const r = e.getBoundingClientRect();
      const filled = !transparent(s.backgroundColor);
      const bordered = ['Top', 'Right', 'Bottom', 'Left'].some((side) => px(s[`border${side}Width`]) > 0 && s[`border${side}Style`] !== 'none' && !transparent(s[`border${side}Color`]));
      return px(s.borderTopLeftRadius) >= 12 && r.width >= 120 && r.height >= 60 && (filled || bordered || s.boxShadow !== 'none') && !e.closest('a, button');
    })
    .map((e) => {
      const s = cs(e);
      return {
        path: describe(e),
        marked: e.hasAttribute('data-card'),
        borders: ['Top', 'Right', 'Bottom', 'Left'].map((side) => ({ w: px(s[`border${side}Width`]), style: s[`border${side}Style`], color: s[`border${side}Color`] })),
        boxShadow: s.boxShadow,
        bg: s.backgroundColor,
        radius: px(s.borderTopLeftRadius),
        padding: [px(s.paddingTop), px(s.paddingRight), px(s.paddingBottom), px(s.paddingLeft)],
      };
    });

  // Drop shadows and borders anywhere (rules 12, 19).
  out.shadows = all.filter((e) => cs(e).boxShadow !== 'none' && isVisible(e)).map((e) => ({ path: describe(e), boxShadow: cs(e).boxShadow }));
  out.borders = all
    .filter((e) => isVisible(e) && !['INPUT', 'TEXTAREA', 'SELECT'].includes(e.tagName))
    .flatMap((e) => {
      const s = cs(e);
      return ['Top', 'Right', 'Bottom', 'Left']
        .filter((side) => px(s[`border${side}Width`]) > 0 && s[`border${side}Style`] !== 'none' && !transparent(s[`border${side}Color`]))
        .map((side) => ({ path: describe(e), tag: e.tagName.toLowerCase(), side: side.toLowerCase(), w: px(s[`border${side}Width`]), style: s[`border${side}Style`], color: s[`border${side}Color`] }));
    });

  // Full-width masks and clip paths, the other way wave dividers are drawn (rule 19).
  out.masks = all
    .filter((e) => {
      const s = cs(e);
      const masked = (s.maskImage && s.maskImage !== 'none') || (s.webkitMaskImage && s.webkitMaskImage !== 'none') || /polygon|path\(/.test(s.clipPath);
      return masked && isVisible(e) && e.getBoundingClientRect().width >= 0.9 * vw;
    })
    .map((e) => ({ path: describe(e), mask: (cs(e).maskImage || cs(e).webkitMaskImage || '').slice(0, 80), clipPath: cs(e).clipPath.slice(0, 80), h: e.getBoundingClientRect().height }));

  // Canvases (rules 15, 16, 17).
  out.canvases = [...document.querySelectorAll('canvas')].map((c) => {
    const s = cs(c);
    const p = c.parentElement ? cs(c.parentElement) : null;
    return {
      path: describe(c),
      attrW: c.width,
      attrH: c.height,
      rect: rect(c),
      imageRendering: s.imageRendering,
      ariaHidden: !!c.closest('[aria-hidden="true"]'),
      transition: { self: { property: s.transitionProperty, duration: s.transitionDuration }, parent: p ? { property: p.transitionProperty, duration: p.transitionDuration } : null },
      opacity: s.opacity,
      section: c.closest('section, header, [data-scene], main > *') ? describe(c.closest('section, header, [data-scene], main > *')) : null,
    };
  });

  // Images and inline SVG (rules 18, 22).
  out.imgs = [...document.querySelectorAll('img')].map((i) => ({ path: describe(i), src: i.currentSrc || i.getAttribute('src'), alt: i.getAttribute('alt'), w: i.getBoundingClientRect().width, naturalW: i.naturalWidth, decorativeRole: i.getAttribute('role') }));
  out.svgs = [...document.querySelectorAll('svg')]
    .filter((s) => !s.parentElement?.closest('svg') && isVisible(s))
    .map((s) => {
      const holder = s.closest('a, button, label, li, p, span, dt, dd, h1, h2, h3, h4, h5, h6, figcaption, td, th');
      return {
        path: describe(s),
        brandSource: s.getAttribute('data-brand-source'),
        ariaHidden: s.getAttribute('aria-hidden') === 'true' || !!s.closest('[aria-hidden="true"]'),
        role: s.getAttribute('role'),
        ariaLabel: s.getAttribute('aria-label'),
        w: s.getBoundingClientRect().width,
        h: s.getBoundingClientRect().height,
        classes: s.getAttribute('class') ?? '',
        nearText: holder ? holder.innerText.replace(/\s+/g, ' ').trim().slice(0, 60) : '',
      };
    });

  // Footer (rule 27).
  const footers = [...document.querySelectorAll('footer')].filter((f) => !f.closest('main, article'));
  const footer = footers.at(-1);
  out.footer = footer
    ? {
        path: describe(footer),
        paragraphs: [...footer.querySelectorAll('p')].filter(isVisible).map((p) => p.innerText.replace(/\s+/g, ' ').trim()),
        text: footer.innerText.replace(/\s+/g, ' ').trim(),
        linkLists: [...footer.querySelectorAll('ul, ol')].filter((l) => l.querySelectorAll('a').length >= 3).length,
      }
    : null;

  // Spacing (rule 10): every margin, padding and gap that is not a multiple of 4 px.
  const offenders = [];
  for (const e of all) {
    if (!isVisible(e) || isSrOnly(e)) continue;
    const s = cs(e);
    const check = (prop, value, allowTen = false) => {
      const v = px(value);
      if (v === null || Number.isNaN(v) || Math.abs(v) < 0.01) return;
      const r = Math.abs(v) % 4;
      if (r < 0.05 || 4 - r < 0.05) return;
      if (allowTen && Math.abs(Math.abs(v) - 10) < 0.05) return;
      offenders.push({ path: describe(e), prop, value: Math.round(v * 100) / 100 });
    };
    const ml = px(s.marginLeft);
    const mr = px(s.marginRight);
    const centred = ml > 0 && Math.abs(ml - mr) < 1;
    for (const side of ['Top', 'Bottom']) check(`margin-${side.toLowerCase()}`, s[`margin${side}`]);
    if (!centred) for (const side of ['Left', 'Right']) check(`margin-${side.toLowerCase()}`, s[`margin${side}`]);
    for (const side of ['Top', 'Right', 'Bottom', 'Left']) check(`padding-${side.toLowerCase()}`, s[`padding${side}`]);
    if (/flex|grid/.test(s.display)) {
      check('row-gap', s.rowGap, true);
      check('column-gap', s.columnGap, true);
    }
  }
  out.spacing = offenders;

  // Layout container (rule 10): the left edge of the page h1.
  const h1 = document.querySelector('main h1, h1');
  out.h1Left = h1 ? h1.getBoundingClientRect().left : null;
  out.mainLeft = document.querySelector('main')?.getBoundingClientRect().left ?? null;

  // Entrance motion (rule 20).
  out.animations = all
    .filter((e) => cs(e).animationName && cs(e).animationName !== 'none')
    .map((e) => {
      const s = cs(e);
      return { path: describe(e), name: s.animationName, duration: s.animationDuration, timing: s.animationTimingFunction, timeline: s.animationTimeline, delay: s.animationDelay, iteration: s.animationIterationCount, belowFold: e.getBoundingClientRect().top + scrollY > vh };
    });
  out.runningAnimations = document.getAnimations().filter((a) => a.playState === 'running').length;

  // The hero (rules 17, 25): the section that holds the first scene canvas, else main's first section.
  const firstCanvas = document.querySelector('main canvas, body > canvas, header ~ canvas, [data-scene] canvas');
  const hero = firstCanvas?.closest('section, [data-hero]') ?? document.querySelector('main section, main > *');
  if (hero) {
    const heroH1 = hero.querySelector('h1');
    let lede = null;
    if (heroH1) {
      const ps = [...hero.querySelectorAll('p')].filter((p) => isVisible(p) && heroH1.compareDocumentPosition(p) & Node.DOCUMENT_POSITION_FOLLOWING);
      lede = ps[0] ? { text: ps[0].innerText.trim(), words: ps[0].innerText.trim().split(/\s+/).filter(Boolean).length } : null;
    }
    const content = [...hero.querySelectorAll('h1, h2, p, a, button, input, [data-card], li')].filter((e) => isVisible(e) && !isSrOnly(e));
    out.hero = {
      path: describe(hero),
      rect: rect(hero),
      hasH1: !!heroH1,
      text: hero.innerText.replace(/\s+/g, ' ').trim().slice(0, 600),
      lede,
      contentBottom: content.length ? Math.max(...content.map((e) => e.getBoundingClientRect().bottom + scrollY)) : null,
      canvases: [...hero.querySelectorAll('canvas')].map((c) => rect(c)),
    };
  } else out.hero = null;

  return out;
}
