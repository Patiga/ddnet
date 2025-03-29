#include "backend_wgpu.h"

#include "base/system.h"

#include <engine/client/backend_sdl.h>

#include <SDL_syswm.h>

CCommandProcessorFragment_GLBase *CreateWGPUCommandProcessorFragment(int WgpuBackendType)
{
	return new CCommandProcessorFragment_WGPU(WgpuBackendType);
}

ERunCommandReturnTypes CCommandProcessorFragment_WGPU::RunCommand(const CCommandBuffer::SCommand *pBaseCommand)
{
	switch(pBaseCommand->m_Cmd)
	{
	case CCommandProcessorFragment_WGPU::CMD_PRE_INIT: Cmd_PreInit(static_cast<const SCommand_PreInit *>(pBaseCommand)); break;
	case CCommandProcessorFragment_WGPU::CMD_INIT: Cmd_Init(static_cast<const SCommand_Init *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_UPDATE_VIEWPORT: Cmd_UpdateViewport(static_cast<const CCommandBuffer::SCommand_Update_Viewport *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_SWAP: Cmd_Swap(static_cast<const CCommandBuffer::CCommandBuffer::SCommand_Swap *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_CLEAR: Cmd_Clear(static_cast<const CCommandBuffer::SCommand_Clear *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_RENDER: Cmd_Render(static_cast<const CCommandBuffer::SCommand_Render *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_TEXTURE_CREATE: Cmd_Texture_Create(static_cast<const CCommandBuffer::SCommand_Texture_Create *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_TEXTURE_DESTROY: Cmd_Texture_Destroy(static_cast<const CCommandBuffer::SCommand_Texture_Destroy *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_TEXT_TEXTURES_CREATE: Cmd_TextTextures_Create(static_cast<const CCommandBuffer::SCommand_TextTextures_Create *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_TEXT_TEXTURES_DESTROY: Cmd_TextTextures_Destroy(static_cast<const CCommandBuffer::SCommand_TextTextures_Destroy *>(pBaseCommand)); break;
	case CCommandBuffer::CMD_TEXT_TEXTURE_UPDATE: Cmd_TextTexture_Update(static_cast<const CCommandBuffer::SCommand_TextTexture_Update *>(pBaseCommand)); break;
	}
	return ERunCommandReturnTypes::RUN_COMMAND_COMMAND_HANDLED;
}

bool CCommandProcessorFragment_WGPU::Cmd_PreInit(const SCommand_PreInit *pCommand)
{
	SDL_SysWMinfo WindowInfo;
	SDL_GetVersion(&WindowInfo.version);
	if(SDL_GetWindowWMInfo(pCommand->m_pWindow, &WindowInfo) == 0)
	{
		dbg_msg("wgpu", "error from sdl: %s", SDL_GetError());
		return false;
	}
	Rust()->init_window((uint8_t *)&WindowInfo, pCommand->m_Width, pCommand->m_Height);
	return true;
}

bool CCommandProcessorFragment_WGPU::Cmd_Init(const SCommand_Init *pCommand)
{
	pCommand->m_pCapabilities->m_TileBuffering = false;
	pCommand->m_pCapabilities->m_QuadBuffering = false;
	pCommand->m_pCapabilities->m_TextBuffering = false;
	pCommand->m_pCapabilities->m_QuadContainerBuffering = false;

	pCommand->m_pCapabilities->m_MipMapping = false;
	pCommand->m_pCapabilities->m_NPOTTextures = false;
	pCommand->m_pCapabilities->m_3DTextures = false;
	pCommand->m_pCapabilities->m_2DArrayTextures = false;
	pCommand->m_pCapabilities->m_2DArrayTexturesAsExtension = false;
	pCommand->m_pCapabilities->m_ShaderSupport = false;

	pCommand->m_pCapabilities->m_TrianglesAsQuads = false;

	pCommand->m_pCapabilities->m_ContextMajor = 0;
	pCommand->m_pCapabilities->m_ContextMinor = 0;
	pCommand->m_pCapabilities->m_ContextPatch = 0;
	return true;
}

void CCommandProcessorFragment_WGPU::Cmd_UpdateViewport(const CCommandBuffer::SCommand_Update_Viewport *pCommand)
{
	dbg_assert(pCommand->m_X >= 0 && pCommand->m_Y >= 0, "Negative view port pos");
	dbg_assert(pCommand->m_Width >= 0 && pCommand->m_Height >= 0, "Negative view port size");
	Rust()->update_viewport(pCommand->m_X, pCommand->m_Y, pCommand->m_Width, pCommand->m_Height, pCommand->m_ByResize);
}

void CCommandProcessorFragment_WGPU::Cmd_Swap(const CCommandBuffer::SCommand_Swap *pCommand)
{
	Rust()->swap();
}

void CCommandProcessorFragment_WGPU::Cmd_Clear(const CCommandBuffer::SCommand_Clear *pCommand)
{
	Rust()->clear(
		pCommand->m_Color.r,
		pCommand->m_Color.g,
		pCommand->m_Color.b,
		pCommand->m_Color.a);
}

void CCommandProcessorFragment_WGPU::Cmd_Render(const CCommandBuffer::SCommand_Render *pCommand)
{
	if(pCommand->m_State.m_ClipEnable)
	{
		dbg_assert(pCommand->m_State.m_ClipX >= 0 && pCommand->m_State.m_ClipY >= 0, "Invalid clip pos");
		dbg_assert(pCommand->m_State.m_ClipW >= 0 && pCommand->m_State.m_ClipH >= 0, "Invalid clip pos");
	}
	dbg_assert(pCommand->m_PrimCount > 0, "Zero or negative primitive count");
	size_t VertexCount;
	switch(pCommand->m_PrimType)
	{
	case EPrimitiveType::LINES:
		VertexCount = pCommand->m_PrimCount * 2;
		break;
	case EPrimitiveType::TRIANGLES:
		VertexCount = pCommand->m_PrimCount * 3;
		break;
	case EPrimitiveType::QUADS:
		VertexCount = pCommand->m_PrimCount * 4;
		break;
	default:
		return;
	};
	size_t VertexBytes = VertexCount * sizeof(CCommandBuffer::SVertex);
	Rust()->render(
		(int)pCommand->m_State.m_BlendMode,
		(int)pCommand->m_State.m_WrapMode,
		pCommand->m_State.m_Texture,
		pCommand->m_State.m_ScreenTL.x,
		pCommand->m_State.m_ScreenTL.y,
		pCommand->m_State.m_ScreenBR.x,
		pCommand->m_State.m_ScreenBR.y,
		pCommand->m_State.m_ClipEnable,
		pCommand->m_State.m_ClipX,
		pCommand->m_State.m_ClipY,
		pCommand->m_State.m_ClipW,
		pCommand->m_State.m_ClipH,
		(int)pCommand->m_PrimType,
		pCommand->m_PrimCount,
		rust::Slice(reinterpret_cast<const uint8_t *>(pCommand->m_pVertices), VertexBytes));
}

void CCommandProcessorFragment_WGPU::Cmd_Texture_Create(const CCommandBuffer::SCommand_Texture_Create *pCommand)
{
	dbg_assert(pCommand->m_Width > 0, "Texture width <= 0");
	dbg_assert(pCommand->m_Height > 0, "Texture height <= 0");
	Rust()->create_texture(
		pCommand->m_Slot,
		4, // RGBA
		pCommand->m_Flags,
		(uint32_t)pCommand->m_Width,
		(uint32_t)pCommand->m_Height,
		rust::Slice<const uint8_t>(pCommand->m_pData, pCommand->m_Width * pCommand->m_Height * 4));
	free(pCommand->m_pData);
}

void CCommandProcessorFragment_WGPU::Cmd_Texture_Destroy(const CCommandBuffer::SCommand_Texture_Destroy *pCommand)
{
	Rust()->destroy_texture(pCommand->m_Slot);
}

void CCommandProcessorFragment_WGPU::Cmd_TextTextures_Create(const CCommandBuffer::SCommand_TextTextures_Create *pCommand)
{
	dbg_assert(pCommand->m_Width > 0, "Texture width <= 0");
	dbg_assert(pCommand->m_Height > 0, "Texture height <= 0");
	Rust()->create_texture(
		pCommand->m_Slot,
		1, // R (single channel)
		TextureFlag::NO_MIPMAPS,
		(uint32_t)pCommand->m_Width,
		(uint32_t)pCommand->m_Height,
		rust::Slice<const uint8_t>(pCommand->m_pTextData, pCommand->m_Width * pCommand->m_Height));
	Rust()->create_texture(
		pCommand->m_SlotOutline,
		1,
		TextureFlag::NO_MIPMAPS,
		(uint32_t)pCommand->m_Width,
		(uint32_t)pCommand->m_Height,
		rust::Slice<const uint8_t>(pCommand->m_pTextOutlineData, pCommand->m_Width * pCommand->m_Height));
	free(pCommand->m_pTextData);
	free(pCommand->m_pTextOutlineData);
}

void CCommandProcessorFragment_WGPU::Cmd_TextTextures_Destroy(const CCommandBuffer::SCommand_TextTextures_Destroy *pCommand)
{
	Rust()->destroy_texture(pCommand->m_Slot);
	Rust()->destroy_texture(pCommand->m_SlotOutline);
}

void CCommandProcessorFragment_WGPU::Cmd_TextTexture_Update(const CCommandBuffer::SCommand_TextTexture_Update *pCommand)
{
	dbg_assert(pCommand->m_Width > 0, "Texture update width <= 0");
	dbg_assert(pCommand->m_Height > 0, "Texture update height <= 0");
	dbg_assert(pCommand->m_X >= 0, "Texture update x < 0");
	dbg_assert(pCommand->m_Y >= 0, "Texture update y < 0");
	Rust()->update_texture(
		pCommand->m_Slot,
		(uint32_t)pCommand->m_X,
		(uint32_t)pCommand->m_Y,
		(uint32_t)pCommand->m_Width,
		(uint32_t)pCommand->m_Height,
		rust::Slice<const uint8_t>(pCommand->m_pData, pCommand->m_Width * pCommand->m_Height));
	free(pCommand->m_pData);
}
