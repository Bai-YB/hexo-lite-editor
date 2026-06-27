<script lang="ts">
  import { ImageOff, Search } from "@lucide/svelte";
  import type { Post } from "$lib/types/post";

  export let posts: Post[] = [];
  export let activePath = "";
  export let showCover = true;
  export let coverSourcePriority: string[] = ["cover"];
  export let onSelect: (post: Post) => void = () => {};
  export let onNewPost: () => void = () => {};

  let query = "";
  let category = "";
  let tag = "";
  let failedImages = new Set<string>();

  $: categories = Array.from(new Set(posts.flatMap((post) => post.categories))).filter(Boolean).sort();
  $: tags = Array.from(new Set(posts.flatMap((post) => post.tags))).filter(Boolean).sort();
  $: filteredPosts = posts.filter((post) => {
    const haystack = `${post.title} ${post.fileName} ${post.tags.join(" ")} ${post.categories.join(" ")}`.toLowerCase();
    return (
      haystack.includes(query.trim().toLowerCase()) &&
      (!category || post.categories.includes(category)) &&
      (!tag || post.tags.includes(tag))
    );
  });

  function coverFor(post: Post): string | undefined {
    for (const source of coverSourcePriority) {
      const value =
        source === "cover"
          ? post.cover
          : source === "top_img" || source === "topImg"
            ? post.topImg
            : source === "thumbnail"
              ? post.thumbnail
              : source === "index_img" || source === "indexImg"
                ? post.indexImg
                : source === "banner"
                  ? post.banner
                  : undefined;
      if (value && !failedImages.has(`${post.filePath}:${value}`)) return value;
    }
    return undefined;
  }

  function markImageFailed(post: Post, url: string) {
    failedImages = new Set([...failedImages, `${post.filePath}:${url}`]);
  }
</script>

<aside class="post-list">
  <div class="panel-header">
    <div>
      <h2>文章</h2>
      <span>{filteredPosts.length} / {posts.length}</span>
    </div>
    <button on:click={onNewPost}>新建</button>
  </div>

  <label class="search-box">
    <Search size={16} />
    <input bind:value={query} placeholder="搜索标题、文件、标签" />
  </label>

  <div class="filters">
    <select bind:value={category}>
      <option value="">全部分类</option>
      {#each categories as item}
        <option value={item}>{item}</option>
      {/each}
    </select>
    <select bind:value={tag}>
      <option value="">全部标签</option>
      {#each tags as item}
        <option value={item}>{item}</option>
      {/each}
    </select>
  </div>

  <div class="post-items">
    {#if filteredPosts.length === 0}
      <div class="empty-state">当前没有可显示的 Markdown 文章。</div>
    {/if}
    {#each filteredPosts as post}
      {@const cover = coverFor(post)}
      <button class:active={post.filePath === activePath} class:with-cover={showCover} class="post-item" on:click={() => onSelect(post)}>
        {#if showCover}
          <span class="post-cover" aria-hidden="true">
            {#if cover}
              <img src={cover} alt="" on:error={() => markImageFailed(post, cover)} />
            {:else}
              <ImageOff size={18} />
            {/if}
          </span>
        {/if}
        <span class="post-summary">
          <strong>{post.title}</strong>
          <span>{post.fileName}</span>
          <small>{post.date || post.updatedAt || "未设置日期"}</small>
          <span class="chips">
            {#each post.categories.slice(0, 2) as item}
              <em>{item}</em>
            {/each}
            {#each post.tags.slice(0, 3) as item}
              <em>{item}</em>
            {/each}
          </span>
        </span>
      </button>
    {/each}
  </div>
</aside>
